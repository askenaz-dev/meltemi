// SPDX-License-Identifier: Apache-2.0

//! Newline-delimited JSON framing (adaptadores-propios-acp task 1.2).
//!
//! Both provider dialects use the same framing on the wire — one JSON value
//! per line over the subprocess's stdio — so both adapters share it. The
//! reader and the writer are generic over any async byte stream, which is what
//! lets the bridge's tests drive a whole conversation in memory, with no
//! process and no platform quirks involved.
//!
//! Two decisions worth naming:
//!
//! - **A line that does not parse is kept, not dropped.** Provider CLIs print
//!   the occasional non-JSON line (a warning, a banner). Swallowing it would
//!   erase evidence exactly when a session is going wrong, so it surfaces as
//!   [`Frame::Unparsed`] and the dialect decides.
//! - **Every write flushes.** The provider is waiting on a line, not on a
//!   buffer: a turn that sits unflushed looks to the human like an agent that
//!   went quiet.

use serde_json::Value;
use tokio::io::{AsyncBufReadExt, AsyncRead, AsyncWrite, AsyncWriteExt, BufReader, Lines};

/// One line read from the provider.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Frame {
    /// A line that parsed as JSON.
    Json(Value),
    /// A line that did not parse as JSON, kept verbatim.
    Unparsed(String),
}

impl Frame {
    /// The parsed value, when the line was JSON.
    #[must_use]
    pub fn json(&self) -> Option<&Value> {
        match self {
            Frame::Json(value) => Some(value),
            Frame::Unparsed(_) => None,
        }
    }
}

/// Reads newline-delimited JSON from a provider's output stream.
pub struct FrameReader<R> {
    lines: Lines<BufReader<R>>,
}

impl<R: AsyncRead + Unpin> FrameReader<R> {
    /// Frames an output stream.
    pub fn new(reader: R) -> Self {
        Self {
            lines: BufReader::new(reader).lines(),
        }
    }

    /// The next frame, or `None` at end of stream.
    ///
    /// Blank lines are not frames: they carry nothing and some CLIs emit them
    /// between messages.
    ///
    /// # Errors
    ///
    /// Returns the underlying I/O error when the stream fails.
    pub async fn next_frame(&mut self) -> std::io::Result<Option<Frame>> {
        while let Some(line) = self.lines.next_line().await? {
            if line.trim().is_empty() {
                continue;
            }
            return Ok(Some(match serde_json::from_str::<Value>(&line) {
                Ok(value) => Frame::Json(value),
                Err(_) => Frame::Unparsed(line),
            }));
        }
        Ok(None)
    }
}

/// Writes newline-delimited JSON to a provider's input stream.
pub struct FrameWriter<W> {
    writer: W,
}

impl<W: AsyncWrite + Unpin> FrameWriter<W> {
    /// Frames an input stream.
    pub fn new(writer: W) -> Self {
        Self { writer }
    }

    /// Writes one value as a line and flushes it.
    ///
    /// # Errors
    ///
    /// Returns the underlying I/O error when the write or the flush fails.
    pub async fn write_frame(&mut self, value: &Value) -> std::io::Result<()> {
        // `to_string` never emits a raw newline, so one value is always one
        // line: the framing cannot be broken by the payload.
        let mut line = serde_json::to_string(value)?;
        line.push('\n');
        self.writer.write_all(line.as_bytes()).await?;
        self.writer.flush().await
    }

    /// Closes the input stream: for both provider wires, end of input is the
    /// polite way to say the conversation is over.
    ///
    /// # Errors
    ///
    /// Returns the underlying I/O error when the close fails.
    pub async fn close(&mut self) -> std::io::Result<()> {
        self.writer.shutdown().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[tokio::test]
    async fn frames_travel_whole_in_both_directions() {
        // The bridge's whole conversation with a provider is this: values in,
        // lines out. Driven in memory — no process, no platform in the way.
        let (adapter_side, provider_side) = tokio::io::duplex(1024);
        let mut writer = FrameWriter::new(adapter_side);
        let mut reader = FrameReader::new(provider_side);

        writer
            .write_frame(&json!({"type": "user", "text": "hola"}))
            .await
            .unwrap();
        writer
            .write_frame(&json!({"jsonrpc": "2.0", "id": 1}))
            .await
            .unwrap();
        writer.close().await.unwrap();

        assert_eq!(
            reader.next_frame().await.unwrap(),
            Some(Frame::Json(json!({"type": "user", "text": "hola"})))
        );
        assert_eq!(
            reader.next_frame().await.unwrap(),
            Some(Frame::Json(json!({"jsonrpc": "2.0", "id": 1})))
        );
        assert_eq!(
            reader.next_frame().await.unwrap(),
            None,
            "a closed input reads as end of stream, not as an error"
        );
    }

    #[tokio::test]
    async fn a_line_that_is_not_json_is_surfaced_instead_of_swallowed() {
        // Provider CLIs print banners and warnings. Dropping them would erase
        // evidence exactly when a session is going wrong.
        let (mut adapter_side, provider_side) = tokio::io::duplex(1024);
        tokio::io::AsyncWriteExt::write_all(
            &mut adapter_side,
            b"starting up\n\n{\"type\":\"result\"}\n",
        )
        .await
        .unwrap();
        drop(adapter_side);

        let mut reader = FrameReader::new(provider_side);
        assert_eq!(
            reader.next_frame().await.unwrap(),
            Some(Frame::Unparsed("starting up".into()))
        );
        assert_eq!(
            reader.next_frame().await.unwrap(),
            Some(Frame::Json(json!({"type": "result"}))),
            "a blank line is skipped and the stream keeps its footing"
        );
        assert_eq!(reader.next_frame().await.unwrap(), None);
    }
}
