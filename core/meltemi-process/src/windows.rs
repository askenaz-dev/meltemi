// SPDX-License-Identifier: Apache-2.0

//! The Windows scope: a Job Object that kills what it holds when it closes.

use std::io;

use windows_sys::Win32::Foundation::{CloseHandle, HANDLE};
use windows_sys::Win32::System::JobObjects::{
    AssignProcessToJobObject, CreateJobObjectW, JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
    JOBOBJECT_EXTENDED_LIMIT_INFORMATION, JobObjectExtendedLimitInformation,
    SetInformationJobObject, TerminateJobObject,
};
use windows_sys::Win32::System::Threading::{OpenProcess, PROCESS_SET_QUOTA, PROCESS_TERMINATE};

/// The exit code reported for the processes an ended scope takes with it.
///
/// `1` rather than `0`: these processes did not finish, they were ended, and a
/// success code would tell anyone reading the exit status the opposite.
const ENDED_EXIT_CODE: u32 = 1;

/// An open job object.
///
/// Holds the only handle. Closing it is what binds the lives of everything
/// inside to the life of this process, so it is closed exactly once, in `Drop`.
#[derive(Debug)]
pub struct Scope(HANDLE);

// The handle is an opaque kernel object usable from any thread; nothing in this
// type is thread-affine. Rust cannot know that about a raw pointer, so it is
// asserted here, where the reason can be read.
unsafe impl Send for Scope {}
unsafe impl Sync for Scope {}

impl Scope {
    pub fn new() -> io::Result<Self> {
        // An unnamed job: nothing else has a name to open it by, which is what
        // keeps one session's scope out of reach of another's.
        // SAFETY: both arguments are the documented "no attributes, no name"
        // null pointers, and the return value is checked before it is used.
        let handle = unsafe { CreateJobObjectW(std::ptr::null(), std::ptr::null()) };
        if handle.is_null() {
            return Err(io::Error::last_os_error());
        }
        let scope = Self(handle);

        // The whole point: when the last handle to this job closes — including
        // when the system closes it because this process died without running
        // anything — every process still inside is terminated.
        let mut limits = JOBOBJECT_EXTENDED_LIMIT_INFORMATION::default();
        limits.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
        // SAFETY: the handle is one this call just created and still owns; the
        // pointer and length describe the matching struct for the information
        // class being set.
        let set = unsafe {
            SetInformationJobObject(
                scope.0,
                JobObjectExtendedLimitInformation,
                std::ptr::from_ref(&limits).cast(),
                u32::try_from(size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>())
                    .expect("the limit struct is far smaller than u32::MAX"),
            )
        };
        if set == 0 {
            // `scope` is dropped here and its handle closed: a job without its
            // limit is not the thing this type promises to be, and leaving it
            // open would leak a handle on the way out of a failure.
            return Err(io::Error::last_os_error());
        }
        Ok(scope)
    }

    pub fn adopt_pid(&self, pid: u32) -> io::Result<()> {
        // The two rights `AssignProcessToJobObject` documents, and no others:
        // this crate never reads a provider's memory and never inspects its
        // tokens.
        // SAFETY: a plain call with a checked return; the handle it yields is
        // closed below on both paths.
        let process = unsafe { OpenProcess(PROCESS_SET_QUOTA | PROCESS_TERMINATE, 0, pid) };
        if process.is_null() {
            return Err(io::Error::last_os_error());
        }
        // SAFETY: both handles are open and owned here — the job by `self`, the
        // process by the call above.
        let assigned = unsafe { AssignProcessToJobObject(self.0, process) };
        let outcome = if assigned == 0 {
            Err(io::Error::last_os_error())
        } else {
            Ok(())
        };
        // SAFETY: closing a handle this function opened and no longer uses. The
        // process stays in the job: membership belongs to the job, not to this
        // handle.
        unsafe {
            let _ = CloseHandle(process);
        }
        outcome
    }

    pub fn end(&self) -> io::Result<()> {
        // SAFETY: the handle is open and owned by `self` for as long as `self`
        // exists.
        let ended = unsafe { TerminateJobObject(self.0, ENDED_EXIT_CODE) };
        if ended == 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(())
    }
}

impl Drop for Scope {
    fn drop(&mut self) {
        // This is the second half of the guarantee, and the half a crash still
        // honours: closing the last handle terminates whatever is left inside.
        // SAFETY: closing a handle owned by this value, exactly once, at the
        // end of its life.
        unsafe {
            let _ = CloseHandle(self.0);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::{Command, Stdio};
    use std::time::{Duration, Instant};

    use windows_sys::Win32::Foundation::STATUS_PENDING;
    use windows_sys::Win32::System::Threading::{
        GetExitCodeProcess, PROCESS_QUERY_LIMITED_INFORMATION,
    };

    /// The exit code a process that has not exited reports.
    #[allow(clippy::cast_sign_loss)]
    const STILL_RUNNING: u32 = STATUS_PENDING as u32;

    /// Whether a process id belongs to something still **running**.
    ///
    /// Deliberately not "can it be opened": a process that has terminated but
    /// whose launcher has not reaped it still has a process object, and opening
    /// it succeeds. Asking that question cost this test a false failure before
    /// it asked the right one — whether the exit code is still pending.
    fn alive(pid: u32) -> bool {
        // SAFETY: a plain call with a checked return, whose handle is closed
        // on both paths below.
        let handle = unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid) };
        if handle.is_null() {
            return false;
        }
        let mut code: u32 = 0;
        // SAFETY: the handle is open and owned here, and `code` is a valid
        // out-parameter for the call's lifetime.
        let read = unsafe { GetExitCodeProcess(handle, &raw mut code) };
        // SAFETY: closing a handle this function opened.
        unsafe {
            let _ = CloseHandle(handle);
        }
        read != 0 && code == STILL_RUNNING
    }

    /// The identifiers of every process whose parent is `parent`.
    fn children_of(parent: u32) -> Vec<u32> {
        let out = Command::new("powershell")
            .args([
                "-NoProfile",
                "-Command",
                &format!(
                    "Get-CimInstance Win32_Process -Filter \"ParentProcessId={parent}\" | \
                     ForEach-Object {{ $_.ProcessId }}"
                ),
            ])
            .output()
            .expect("powershell ships with the platform this test only runs on");
        String::from_utf8_lossy(&out.stdout)
            .lines()
            .filter_map(|line| line.trim().parse().ok())
            .collect()
    }

    /// Waits for `condition`, up to `limit`. Returns whether it came true.
    fn within(limit: Duration, mut condition: impl FnMut() -> bool) -> bool {
        let deadline = Instant::now() + limit;
        while Instant::now() < deadline {
            if condition() {
                return true;
            }
            std::thread::sleep(Duration::from_millis(50));
        }
        condition()
    }

    // Scenario: Un lanzador intermedio no deja huérfanos
    #[test]
    fn ending_a_scope_ends_the_grandchild_a_shim_left_behind() {
        // The defect this crate exists for, reproduced and then prevented: a
        // `.cmd` is `cmd.exe` running a script that creates the process doing
        // the work, and ending the shim alone leaves that process running.
        let dir = std::env::temp_dir().join(format!("meltemi-scope-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("a temp directory");
        let shim = dir.join("intermediary.cmd");
        // `ping` waits without doing anything and ships with every Windows;
        // `/t` is not used anywhere, so nothing here depends on a kill utility.
        std::fs::write(&shim, "@echo off\r\nping -n 60 127.0.0.1 >nul\r\n")
            .expect("writing the shim");

        let scope = Scope::new().expect("a scope can be opened");
        let mut child = Command::new(&shim)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .expect("the shim launches");
        let shim_pid = child.id();
        scope
            .adopt_pid(shim_pid)
            .expect("a live child can be adopted");

        // The grandchild the shim creates: `cmd.exe` has to parse the script
        // and call the launcher, so it is not there the instant `spawn`
        // returns.
        let grandchildren = {
            let mut found = Vec::new();
            within(Duration::from_secs(10), || {
                found = children_of(shim_pid);
                !found.is_empty()
            });
            found
        };
        assert!(
            !grandchildren.is_empty(),
            "the shim never created the process this test is about; without it \
             the test proves nothing"
        );

        scope.end().expect("ending a scope with processes in it");

        for pid in &grandchildren {
            assert!(
                within(Duration::from_secs(10), || !alive(*pid)),
                "the grandchild {pid} outlived the scope: ending the shim is \
                 not ending what the shim launched"
            );
        }
        assert!(
            within(Duration::from_secs(10), || !alive(shim_pid)),
            "the adopted process itself outlived the scope"
        );

        let _ = child.wait();
        let _ = std::fs::remove_dir_all(&dir);
    }

    // Scenario: El daemon termina de golpe y nada le sobrevive
    #[test]
    fn dropping_a_scope_ends_what_is_inside_it_without_ending_it_explicitly() {
        // The half a destructor cannot keep and the kernel can: when the last
        // handle closes — which is what the system does to a process that died
        // without running anything — the processes inside are terminated. This
        // test drops the handle instead of crashing, because what it is
        // asserting is the kernel's behaviour on handle closure, which is the
        // same behaviour either way.
        let scope = Scope::new().expect("a scope can be opened");
        let mut child = Command::new("ping")
            .args(["-n", "60", "127.0.0.1"])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .expect("ping ships with the platform");
        let pid = child.id();
        scope.adopt_pid(pid).expect("a live child can be adopted");
        assert!(alive(pid), "the process is running before the scope closes");

        drop(scope);

        assert!(
            within(Duration::from_secs(10), || !alive(pid)),
            "the process outlived the scope's handle: a launcher that dies \
             without cleaning up would leave this running"
        );
        let _ = child.wait();
    }

    #[test]
    fn adopting_an_identifier_that_names_nothing_fails_instead_of_pretending() {
        // A scope that silently accepted an unknown identifier would report a
        // guarantee it is not providing. Identifier 0 is the system idle
        // process, which cannot be opened with these rights.
        let scope = Scope::new().expect("a scope can be opened");
        assert!(
            scope.adopt_pid(0).is_err(),
            "adopting nothing must be an error, not a silent success"
        );
    }
}
