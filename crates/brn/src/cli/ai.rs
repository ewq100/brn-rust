//! Explicit subscription actions; login prompts never enter the output envelope.
use super::{
    error::CliError, expect_positionals, parse_timeout, scan, sub_word, usage, Globals, Scanned,
    Tokens,
};
use brn_workflow::{Provider, ReasoningEffort, Selection};

pub enum AiCommand {
    Connect(Provider, u64),
    Disconnect(Provider),
    Status,
    Models(Provider, u64),
    Select(Selection),
    Effort(Option<ReasoningEffort>),
}

impl AiCommand {
    pub fn name(&self) -> &'static str {
        match self {
            Self::Connect(..) => "ai.connect",
            Self::Disconnect(_) => "ai.disconnect",
            Self::Status => "ai.status",
            Self::Models(..) => "ai.models",
            Self::Select(_) => "ai.select",
            Self::Effort(_) => "ai.effort",
        }
    }
}

pub(super) fn provider(raw: &str) -> Result<Provider, CliError> {
    match raw {
        "chatgpt" => Ok(Provider::Chatgpt),
        "copilot" => Ok(Provider::Copilot),
        _ => Err(usage("provider must be chatgpt|copilot")),
    }
}

pub(super) fn effort(raw: &str) -> Result<ReasoningEffort, CliError> {
    match raw {
        "low" => Ok(ReasoningEffort::Low),
        "medium" => Ok(ReasoningEffort::Medium),
        "high" => Ok(ReasoningEffort::High),
        _ => Err(usage("reasoning effort must be low|medium|high")),
    }
}

pub(super) fn scan_command(
    tokens: Tokens<'_>,
    g: &mut Globals,
    name: &mut Option<&'static str>,
) -> Result<Scanned, CliError> {
    let sub = sub_word(
        tokens,
        "ai",
        "connect|disconnect|status|models|select|effort",
    )?;
    let (label, options): (&str, &[(&str, bool)]) = match sub.as_str() {
        "connect" => ("ai.connect", &[("timeout-seconds", true)]),
        "disconnect" => ("ai.disconnect", &[]),
        "status" => ("ai.status", &[]),
        "models" => ("ai.models", &[("timeout-seconds", true)]),
        "select" => ("ai.select", &[("provider", true), ("model", true)]),
        "effort" => ("ai.effort", &[]),
        _ => return Err(usage("unknown ai subcommand")),
    };
    *name = Some(label);
    scan(tokens, g, options)
}

pub(super) fn parse_command(name: &str, s: &Scanned) -> Result<AiCommand, CliError> {
    if name == "ai.effort" {
        if s.positionals.len() > 1 {
            return Err(usage("ai effort accepts at most one effort"));
        }
        return Ok(AiCommand::Effort(
            s.positionals.first().map(|raw| effort(raw)).transpose()?,
        ));
    }
    if name == "ai.status" {
        expect_positionals(s, 0)?;
        return Ok(AiCommand::Status);
    }
    if name == "ai.select" {
        expect_positionals(s, 0)?;
        let provider = provider(
            s.value("provider")
                .ok_or_else(|| usage("missing --provider"))?,
        )?;
        let model = s.value("model").ok_or_else(|| usage("missing --model"))?;
        let selection = Selection {
            provider,
            model: model.into(),
        };
        selection.validate().map_err(|_| {
            usage("unsupported model; use ai models and explicitly select a supported model")
        })?;
        return Ok(AiCommand::Select(selection));
    }
    expect_positionals(s, 1)?;
    let provider = provider(
        s.positionals
            .first()
            .ok_or_else(|| usage("missing provider"))?,
    )?;
    let timeout = s
        .value("timeout-seconds")
        .map(parse_timeout)
        .transpose()?
        .unwrap_or(300);
    Ok(match name {
        "ai.connect" => AiCommand::Connect(provider, timeout),
        "ai.disconnect" => AiCommand::Disconnect(provider),
        "ai.models" => AiCommand::Models(provider, timeout),
        _ => unreachable!(),
    })
}
