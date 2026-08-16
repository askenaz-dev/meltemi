// SPDX-License-Identifier: Apache-2.0

//! Relaying the CLI's permission questions to the permission proxy
//! (adaptadores-propios-acp task 3.3, design D5).
//!
//! When the piloted CLI wants to run a tool, the adapter has exactly one job:
//! turn that into `session/request_permission` and hand back whatever the proxy
//! decides. It never decides. It has no rules, no memory of past answers and no
//! opinion — the decision belongs to the proxy meltemid already runs, with the
//! same tray, the same rules and the same log every other agent's requests go
//! through, and the daemon gains no transport whatsoever from this.
//!
//! Three consequences are written down rather than discovered:
//!
//! - **No decision means no.** A proxy that cannot be reached, an outcome this
//!   adapter does not recognise, an option id that is not one of the two it
//!   offered: all of them are a denial. The failure mode of a permission relay
//!   must be denial, never a shrug that reads as consent.
//! - **Only what the provider can honour is offered.** This CLI's permission
//!   channel answers one call at a time: allow this, or refuse this. There is
//!   no "and stop asking" on that wire, so none is offered — an option that
//!   could not be kept would be a promise made to the human in somebody else's
//!   name.
//! - **An allowed call runs exactly as it was asked.** The channel lets the
//!   answer rewrite the tool's arguments. This adapter hands them back
//!   unchanged: a bridge that edited what it was relaying would be deciding
//!   something nobody asked it to decide, and the human approved what they were
//!   shown.
//!
//!   The rule has **exactly one exception, and it is bounded by construction**:
//!   the tool through which this provider asks a person to choose. There the
//!   input *is* the form — the human is not rewriting what the agent was going
//!   to do, they are completing what the agent came to ask — and the answer is
//!   written into the question the agent sent, leaving the rest untouched. Every
//!   other tool still travels byte for byte (preguntas-del-agente design D2).

use agent_client_protocol::schema::v1::{
    ContentBlock, PermissionOption, PermissionOptionKind, RequestPermissionRequest, SessionId,
    TextContent, ToolCallStatus, ToolCallUpdate, ToolCallUpdateFields,
};
use serde_json::{Value, json};

use super::mapping;

/// Allow this one call.
pub const ALLOW_ONCE: &str = "allow-once";
/// Refuse it.
pub const REJECT: &str = "reject";

/// The name of the tool the CLI is told to ask through.
pub const TOOL: &str = "approve";
/// The name the permission MCP server is registered under.
pub const SERVER: &str = "meltemi_permissions";

/// What came back from the proxy.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Decision {
    /// The proxy chose one of the offered options.
    Selected(String),
    /// The turn is being cancelled; the question no longer stands.
    Cancelled,
    /// No decision could be obtained at all.
    Unavailable,
}

/// How the CLI is told to ask: the tool reference its permission flag takes.
#[must_use]
pub fn prompt_tool_reference() -> String {
    format!("mcp__{SERVER}__{TOOL}")
}

/// The MCP server the CLI is told to run so that it has somewhere to ask: this
/// same binary, in shim mode, pointed at the channel back to this adapter.
///
/// The name is Meltemi's own and carries no credential, no rule and no policy —
/// only where to ask. Everything that decides anything lives on the other side
/// of that channel.
#[must_use]
pub fn shim_server(exe: &std::path::Path, channel: &std::path::Path) -> (String, Value) {
    (
        SERVER.to_string(),
        json!({
            "type": "stdio",
            "command": exe.display().to_string(),
            "args": [super::shim::SHIM_ARG, channel.display().to_string()],
        }),
    )
}

/// The permission request the proxy will decide.
///
/// It is addressed to the **same tool call id the call was streamed under**
/// whenever the CLI names one, so a client shows the question against the tool
/// call it already displayed instead of against an anonymous prompt. The
/// arguments travel with it: at the moment of deciding, what the tool would
/// actually do is the thing the human needs.
#[must_use]
pub fn request_for(
    session_id: &SessionId,
    tool: &str,
    tool_use_id: Option<&str>,
    input: &Value,
) -> RequestPermissionRequest {
    let mut fields = ToolCallUpdateFields::new()
        .kind(mapping::kind_of(tool))
        .status(ToolCallStatus::Pending)
        .title(mapping::title_for(tool, input));
    if !input.is_null() {
        fields = fields.content(vec![
            ContentBlock::Text(TextContent::new(input.to_string())).into(),
        ]);
    }
    RequestPermissionRequest::new(
        session_id.clone(),
        // Without an id from the CLI the question still has to be about
        // something: the tool's own name is what the human is looking at.
        ToolCallUpdate::new(tool_use_id.unwrap_or(tool).to_string(), fields),
        vec![
            PermissionOption::new(ALLOW_ONCE, "Allow once", PermissionOptionKind::AllowOnce),
            PermissionOption::new(REJECT, "Reject", PermissionOptionKind::RejectOnce),
        ],
    )
}

/// The permission request that carries a question, with the AGENT'S options.
///
/// Unlike an approval — where the two options are Meltemi's own "allow once" and
/// "reject" — here the options belong to the agent and travel verbatim, labels
/// included. A recommendation the agent wrote into a label is information the
/// human is entitled to see, and rewriting it would be this adapter editing the
/// question on its way to the person it was asked of.
///
/// The option ids are positional (`option-0`, `option-1`, …) because the
/// provider's options carry a label and no id, and a label is not an
/// identifier: two options may read alike, and a label may change between
/// versions of the same question.
#[must_use]
pub fn question_request(
    session_id: &SessionId,
    tool_use_id: Option<&str>,
    question: &Question,
) -> RequestPermissionRequest {
    let fields = ToolCallUpdateFields::new()
        .kind(mapping::kind_of(ASK_TOOL))
        .status(ToolCallStatus::Pending)
        .title(if question.text.is_empty() {
            "the agent is asking".to_string()
        } else {
            question.text.clone()
        })
        .content(vec![
            ContentBlock::Text(TextContent::new(question.text.clone())).into(),
        ]);
    let options = question
        .options
        .iter()
        .enumerate()
        .map(|(at, option)| {
            PermissionOption::new(
                question_option_id(at),
                option.label.clone(),
                // An answer to a question is neither an approval nor a refusal.
                // `AllowOnce` is the closest thing the protocol's vocabulary
                // has to "this one, now" — and the kind is what a surface reads
                // to decide styling, never what decides meaning here.
                PermissionOptionKind::AllowOnce,
            )
        })
        .collect();
    RequestPermissionRequest::new(
        session_id.clone(),
        ToolCallUpdate::new(tool_use_id.unwrap_or(ASK_TOOL).to_string(), fields),
        options,
    )
}

/// The id of the option at `index` of a question.
#[must_use]
pub fn question_option_id(index: usize) -> String {
    format!("option-{index}")
}

/// Which option a decision names, when it names one of a question's.
#[must_use]
pub fn question_choice(decision: &Decision, question: &Question) -> Option<usize> {
    let Decision::Selected(option) = decision else {
        return None;
    };
    (0..question.options.len()).find(|at| question_option_id(*at) == *option)
}

/// The answer to a question, in the shape the CLI's channel takes.
///
/// **The one exception to "an allowed call runs exactly as it was approved"**,
/// and it is bounded to this tool by construction: in a question the input *is*
/// the form. The human is not rewriting what the agent was going to do — they
/// are completing what the agent came to ask (design D2). Every other tool's
/// input still travels byte for byte; see [`payload`].
///
/// The answer is written back into the question the agent sent, under `answer`,
/// leaving the rest of the input untouched. That field name is **the half of
/// this change we do not control**: it belongs to the provider's tool, it is
/// not specified by us, and it can move with a version. When it does, the
/// version conformance requirement is what refuses rather than guesses.
#[must_use]
pub fn question_payload(input: &Value, at: usize, answer: &str) -> Value {
    let mut updated = input.clone();
    if let Some(question) = updated
        .get_mut("questions")
        .and_then(Value::as_array_mut)
        .and_then(|all| all.get_mut(at))
        && let Some(object) = question.as_object_mut()
    {
        object.insert("answer".into(), Value::String(answer.to_string()));
    }
    json!({"behavior": "allow", "updatedInput": updated})
}

/// The answer the CLI gets, from the decision the proxy made.
///
/// The shape is the provider's: a payload the permission tool returns, saying
/// allow with the arguments to use, or deny with a reason. The reason is
/// written for the person who will read it in a transcript, and it never
/// pretends the adapter decided anything.
#[must_use]
pub fn payload(decision: &Decision, input: &Value) -> Value {
    match decision {
        Decision::Selected(option) if option == ALLOW_ONCE => json!({
            "behavior": "allow",
            // Unchanged, always: the human approved what they were shown.
            "updatedInput": input,
        }),
        Decision::Selected(option) if option == REJECT => {
            deny("the request was refused where permissions are decided")
        }
        Decision::Cancelled => deny("the turn was cancelled before this was decided"),
        // An option id nobody offered and no decision at all: the same answer.
        // Anything else would let a gap in the chain read as consent.
        Decision::Selected(_) | Decision::Unavailable => {
            deny("no decision could be obtained, so the request was refused")
        }
    }
}

/// A denial with its reason, in the shape the CLI's permission channel takes.
#[must_use]
pub fn deny(reason: &str) -> Value {
    json!({"behavior": "deny", "message": reason})
}

/// Why a tool cannot be relayed at all, when it cannot.
///
/// Some of this provider's tools do not ask for permission to act — they ask a
/// human a question, in the CLI's own interface, and there is no interface here
/// to ask it in. The provider refuses them itself in a non-interactive session;
/// this adapter refuses them for the same reason and says so, because the one
/// thing it must not do is put such a call to the proxy as though a yes could
/// make it work.
///
/// The list is short and observed rather than guessed: a tool that is missing
/// from it is relayed like any other, which is the safe direction to be wrong
/// in — the proxy still decides, and the CLI still refuses what it cannot do.
#[must_use]
pub fn interactive_only(tool: &str) -> Option<&'static str> {
    INTERACTIVE_ONLY
        .iter()
        .find(|(name, _)| *name == tool)
        .map(|(_, reason)| *reason)
}

/// The tools that cannot be relayed, with the reason each is refused for.
///
/// **Empty today, and that is the finding rather than an oversight.**
/// `AskUserQuestion` was its only entry, refused because "there is no interface
/// here to ask it in". That premise expired: Meltemi *is* that interface, and it
/// now relays the question with the agent's own options
/// (preguntas-del-agente D1).
///
/// The mechanism stays because the provider keeps adding tools and the next one
/// may genuinely need a surface this one does not have. A name absent from here
/// is relayed like any other, which remains the safe direction to be wrong in:
/// the proxy still decides and the CLI still refuses what it cannot do.
pub const INTERACTIVE_ONLY: &[(&str, &str)] = &[];

/// The tool through which this provider asks a person to choose.
pub const ASK_TOOL: &str = "AskUserQuestion";

/// One question the agent is asking.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Question {
    /// The question itself, as the agent wrote it.
    pub text: String,
    /// Its options, with the agent's own labels — verbatim, including whatever
    /// the agent marked as recommended inside the label it wrote.
    pub options: Vec<QuestionOption>,
    /// Whether the agent asked for more than one answer. The relay channel
    /// carries exactly one, so this is reported rather than honoured (D3).
    pub multi_select: bool,
}

/// One option of a question.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QuestionOption {
    /// The agent's own label. Never rewritten: a recommendation the agent put
    /// in its label is information the human is entitled to see.
    pub label: String,
    /// The agent's explanation of the option, when it wrote one.
    pub description: Option<String>,
}

/// Reads the questions out of an `AskUserQuestion` input.
///
/// Returns `None` when the input is not the shape this adapter knows, and that
/// is the whole of the answer to a risk we do not control: the exact shape
/// belongs to the provider and can change with its version, so an unrecognized
/// input is **refused**, never guessed at (design D7). A refusal is visible in
/// the session; a guess would put a question to a human that answers something
/// else.
#[must_use]
pub fn questions_of(input: &Value) -> Option<Vec<Question>> {
    let raw = input.get("questions")?.as_array()?;
    if raw.is_empty() {
        return None;
    }
    let mut out = Vec::with_capacity(raw.len());
    for item in raw {
        let options: Vec<QuestionOption> = item
            .get("options")?
            .as_array()?
            .iter()
            .filter_map(|option| {
                Some(QuestionOption {
                    label: option.get("label")?.as_str()?.to_string(),
                    description: option
                        .get("description")
                        .and_then(Value::as_str)
                        .map(str::to_string),
                })
            })
            .collect();
        // A question with no options is not a question this surface can put:
        // there would be nothing to choose.
        if options.is_empty() {
            return None;
        }
        out.push(Question {
            text: item
                .get("question")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string(),
            options,
            multi_select: item
                .get("multiSelect")
                .and_then(Value::as_bool)
                .unwrap_or(false),
        });
    }
    Some(out)
}

/// What the session shows about a call that was refused.
///
/// A denial the human never sees is a session that quietly did less than it
/// was asked to. It is shown against the call it is about, as a failure with
/// its reason — never hidden, and never dressed up as something that worked.
#[must_use]
pub fn refusal_update(
    tool_use_id: &str,
    reason: &str,
) -> agent_client_protocol::schema::v1::SessionUpdate {
    agent_client_protocol::schema::v1::SessionUpdate::ToolCallUpdate(ToolCallUpdate::new(
        tool_use_id.to_string(),
        ToolCallUpdateFields::new()
            .status(ToolCallStatus::Failed)
            .content(vec![
                ContentBlock::Text(TextContent::new(reason.to_string())).into(),
            ]),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use agent_client_protocol::schema::v1::ToolKind;

    #[test]
    fn the_question_reaches_the_proxy_against_the_call_it_is_about() {
        // Scenario: Permiso decidido por el proxy vigente
        let input = json!({"file_path": "NOTES.md", "content": "hola"});
        let request = request_for(&SessionId::new("s-1"), "Write", Some("toolu_1"), &input);
        assert_eq!(request.session_id.0.as_ref(), "s-1");
        assert_eq!(
            request.tool_call.tool_call_id.0.as_ref(),
            "toolu_1",
            "the same id the call was streamed under, so the human sees what it is about"
        );
        assert_eq!(request.tool_call.fields.kind, Some(ToolKind::Edit));
        assert_eq!(
            request.tool_call.fields.status,
            Some(ToolCallStatus::Pending)
        );
        assert!(
            request.tool_call.fields.content.is_some(),
            "and what the tool would actually do travels with it"
        );

        let anonymous = request_for(&SessionId::new("s-1"), "Bash", None, &json!({}));
        assert_eq!(
            anonymous.tool_call.tool_call_id.0.as_ref(),
            "Bash",
            "a call the CLI did not name is still about something"
        );
    }

    #[test]
    fn no_decision_is_a_denial_and_never_a_shrug() {
        // Scenario: Permiso decidido por el proxy vigente
        //
        // Every way the chain can break ends in the same place. This is the one
        // rule of a permission relay that must never have an exception.
        let input = json!({"command": "rm -rf /"});
        let allowed = payload(&Decision::Selected(ALLOW_ONCE.into()), &input);
        assert_eq!(allowed["behavior"], "allow");
        assert_eq!(
            allowed["updatedInput"], input,
            "an allowed call runs exactly as it was approved"
        );

        for decision in [
            Decision::Selected(REJECT.into()),
            Decision::Selected("something-nobody-offered".into()),
            Decision::Cancelled,
            Decision::Unavailable,
        ] {
            let answer = payload(&decision, &input);
            assert_eq!(answer["behavior"], "deny", "for {decision:?}");
            assert!(
                !answer["message"].as_str().unwrap_or_default().is_empty(),
                "and it says why: {answer}"
            );
        }
    }

    #[test]
    fn only_the_options_this_channel_can_honour_are_offered() {
        // The CLI's permission channel answers one call at a time; there is no
        // "and stop asking" on it, so none is offered. An option that could not
        // be kept would be a promise made in somebody else's name.
        let request = request_for(&SessionId::new("s"), "Bash", None, &json!({}));
        let offered: Vec<&str> = request
            .options
            .iter()
            .map(|option| option.option_id.0.as_ref())
            .collect();
        assert_eq!(offered, vec![ALLOW_ONCE, REJECT]);
        assert!(
            !request
                .options
                .iter()
                .any(|option| option.kind == PermissionOptionKind::AllowAlways
                    || option.kind == PermissionOptionKind::RejectAlways),
            "nothing here can be remembered, so nothing here promises to be"
        );
    }

    #[test]
    fn the_cli_is_told_to_ask_through_the_tool_this_adapter_hosts() {
        assert_eq!(prompt_tool_reference(), "mcp__meltemi_permissions__approve");
    }

    // Scenario: Lo que de verdad no se puede relevar se sigue rehusando
    #[test]
    fn the_refusal_mechanism_survives_the_tool_that_left_it() {
        // `AskUserQuestion` was this list's only entry, refused because a
        // headless session has no interface to ask a person in. Meltemi IS that
        // interface, so the premise expired and the tool left the list — but
        // the mechanism did not, because the provider keeps adding tools and
        // the next one may genuinely need a surface this one lacks.
        assert!(
            interactive_only(ASK_TOOL).is_none(),
            "a question is relayed now, with the agent's own options"
        );
        assert!(
            interactive_only("Bash").is_none(),
            "everything else is relayed like anything else, as it always was"
        );
        // Empty TODAY, and said out loud so an empty list does not read as a
        // mechanism somebody deleted.
        assert!(
            INTERACTIVE_ONLY.is_empty(),
            "nothing is currently known to be unrelayable: {INTERACTIVE_ONLY:?}"
        );
        // And it still works: every name on it answers with its own reason, so
        // the day a name is added the refusal is already wired.
        for (tool, reason) in INTERACTIVE_ONLY {
            assert_eq!(interactive_only(tool), Some(*reason));
            assert_eq!(deny(reason)["behavior"], "deny");
        }
    }

    // Scenario: Una pregunta llega con las opciones del agente
    #[test]
    fn a_question_travels_with_the_agents_own_options_and_labels() {
        let input = json!({
            "questions": [{
                "question": "which route?",
                "multiSelect": false,
                "options": [
                    {"label": "Rewrite it (recommended)", "description": "cleaner"},
                    {"label": "Patch it"}
                ]
            }]
        });
        let questions = questions_of(&input).expect("a shape this adapter knows");
        assert_eq!(questions.len(), 1);
        assert_eq!(questions[0].text, "which route?");
        assert_eq!(
            questions[0].options[0].label, "Rewrite it (recommended)",
            "the agent's label travels verbatim: a recommendation it wrote is              information the human is entitled to see"
        );
        assert_eq!(questions[0].options[1].description, None);

        let request = question_request(&SessionId::new("s-1"), Some("toolu_9"), &questions[0]);
        let names: Vec<&str> = request
            .options
            .iter()
            .map(|option| option.name.as_str())
            .collect();
        assert_eq!(names, ["Rewrite it (recommended)", "Patch it"]);
        assert_eq!(
            request.tool_call.tool_call_id.0.as_ref(),
            "toolu_9",
            "asked against the very call the CLI streamed"
        );
    }

    #[test]
    fn a_shape_this_adapter_does_not_know_is_refused_rather_than_guessed_at() {
        // The half of this we do not control. The input shape belongs to the
        // provider and can move with its version; a guess would put a question
        // to a human that answers something else.
        for unknown in [
            json!({}),
            json!({"questions": []}),
            json!({"questions": [{"question": "which?"}]}),
            json!({"questions": [{"question": "which?", "options": []}]}),
            json!({"prompt": "which route?"}),
        ] {
            assert_eq!(
                questions_of(&unknown),
                None,
                "unrecognised, and therefore refused: {unknown}"
            );
        }
    }

    // Scenario: Solo una pregunta completa su propio input
    #[test]
    fn only_a_question_completes_its_own_input_and_nothing_else_is_touched() {
        // The rule that must not bend: an allowed call runs exactly as it was
        // approved. Pinned here for a tool that is not a question.
        let call = json!({"command": "rm -rf /"});
        let allowed = payload(&Decision::Selected(ALLOW_ONCE.into()), &call);
        assert_eq!(
            allowed["updatedInput"], call,
            "an ordinary call travels byte for byte"
        );

        // And the one exception, bounded to a question: the answer is written
        // into the question the agent sent, and NOTHING else moves. In a
        // question the input is the form — completing it is not rewriting what
        // the agent was going to do.
        let input = json!({
            "context": "keep me",
            "questions": [
                {"question": "which route?", "options": [{"label": "Rewrite it"}]},
                {"question": "and then?", "options": [{"label": "Ship"}]}
            ]
        });
        let answer = question_payload(&input, 0, "Rewrite it");
        let updated = &answer["updatedInput"];
        assert_eq!(answer["behavior"], "allow");
        assert_eq!(updated["questions"][0]["answer"], "Rewrite it");
        assert_eq!(
            updated["questions"][0]["question"], "which route?",
            "the question itself is not rewritten"
        );
        assert!(
            updated["questions"][1].get("answer").is_none(),
            "a question that was not answered gains nothing"
        );
        assert_eq!(updated["context"], "keep me", "and the rest is untouched");
    }

    #[test]
    fn an_option_nobody_offered_answers_a_question_no_more_than_it_answers_a_call() {
        let question = Question {
            text: "which route?".into(),
            options: vec![QuestionOption {
                label: "Rewrite it".into(),
                description: None,
            }],
            multi_select: false,
        };
        assert_eq!(
            question_choice(&Decision::Selected(question_option_id(0)), &question),
            Some(0)
        );
        for wrong in [
            Decision::Selected("option-7".into()),
            Decision::Selected(ALLOW_ONCE.into()),
            Decision::Cancelled,
            Decision::Unavailable,
        ] {
            assert_eq!(
                question_choice(&wrong, &question),
                None,
                "a gap in the chain is not an answer: {wrong:?}"
            );
        }
    }

    #[test]
    fn a_refused_call_is_shown_as_refused_with_its_reason() {
        // Scenario: Interacción no relevable denegada con motivo visible
        let update = refusal_update("toolu_2", "the human said no");
        let shown = serde_json::to_value(&update).expect("an update serializes");
        assert_eq!(shown["toolCallId"], "toolu_2");
        assert_eq!(shown["status"], "failed");
        assert!(
            shown.to_string().contains("the human said no"),
            "the reason is in the session, not only in a log: {shown}"
        );
    }
}
