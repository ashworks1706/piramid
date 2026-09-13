//! The chat template a checkpoint ships in tokenizer_config.json, rendered with minijinja.

use std::path::Path;

use minijinja::{Environment, ErrorKind};
use piramid_core::error::InferenceError;
use serde::{Deserialize, Serialize};

/// One turn of a conversation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChatMessage {
    /// Who wrote the turn: system, user, assistant or tool.
    pub role: String,
    /// What the turn says.
    pub content: String,
}

/// A checkpoint's chat template and the special tokens it refers to.
#[derive(Debug)]
pub struct ChatTemplate {
    environment: Environment<'static>,
    bos_token: String,
    eos_token: String,
    eos_text: Option<String>,
}

const TEMPLATE: &str = "chat";

#[derive(Debug, Deserialize)]
struct TokenizerConfig {
    chat_template: Option<TemplateField>,
    bos_token: Option<TokenField>,
    eos_token: Option<TokenField>,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum TemplateField {
    One(String),
    Named(Vec<NamedTemplate>),
}

#[derive(Debug, Deserialize)]
struct NamedTemplate {
    name: String,
    template: String,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum TokenField {
    Text(String),
    Added { content: String },
}

impl TokenField {
    fn into_text(self) -> String {
        match self {
            TokenField::Text(text) | TokenField::Added { content: text } => text,
        }
    }
}

impl ChatTemplate {
    /// Read the template from tokenizer_config.json in a checkpoint directory.
    pub fn from_dir(dir: &Path) -> Result<Self, InferenceError> {
        let path = dir.join("tokenizer_config.json");
        let text = std::fs::read_to_string(&path)
            .map_err(|e| InferenceError::Load(format!("{}: {e}", path.display())))?;
        Self::from_tokenizer_config(&text)
    }

    /// Parse the text of tokenizer_config.json.
    pub fn from_tokenizer_config(text: &str) -> Result<Self, InferenceError> {
        let config: TokenizerConfig = serde_json::from_str(text)
            .map_err(|e| InferenceError::Load(format!("tokenizer_config.json: {e}")))?;
        let source = match config.chat_template {
            Some(TemplateField::One(source)) => source,
            Some(TemplateField::Named(templates)) => templates
                .into_iter()
                .find(|template| template.name == "default")
                .map(|template| template.template)
                .ok_or_else(|| {
                    InferenceError::Load(
                        "tokenizer_config.json: no chat template is named default".to_string(),
                    )
                })?,
            None => {
                return Err(InferenceError::Load(
                    "tokenizer_config.json has no chat_template".to_string(),
                ))
            }
        };
        let mut environment = Environment::new();
        minijinja_contrib::add_to_environment(&mut environment);
        environment
            .set_unknown_method_callback(minijinja_contrib::pycompat::unknown_method_callback);
        environment.add_function(
            "raise_exception",
            |message: String| -> Result<String, minijinja::Error> {
                Err(minijinja::Error::new(ErrorKind::InvalidOperation, message))
            },
        );
        environment
            .add_template_owned(TEMPLATE, source)
            .map_err(|e| InferenceError::Load(format!("chat template: {e}")))?;
        let eos_text = config.eos_token.map(TokenField::into_text);
        Ok(Self {
            environment,
            bos_token: config
                .bos_token
                .map(TokenField::into_text)
                .unwrap_or_default(),
            eos_token: eos_text.clone().unwrap_or_default(),
            eos_text,
        })
    }

    /// Render a conversation into prompt text, ending with the opening of an assistant turn.
    pub fn render(&self, messages: &[ChatMessage]) -> Result<String, InferenceError> {
        if messages.is_empty() {
            return Err(InferenceError::InvalidRequest(
                "a chat needs at least one message".to_string(),
            ));
        }
        let template = self
            .environment
            .get_template(TEMPLATE)
            .map_err(|e| InferenceError::Runtime(format!("chat template: {e}")))?;
        template
            .render(minijinja::context! {
                messages => messages,
                add_generation_prompt => true,
                bos_token => &self.bos_token,
                eos_token => &self.eos_token,
            })
            .map_err(|e| InferenceError::InvalidRequest(format!("chat template: {e}")))
    }

    /// The end-of-sequence token text the tokenizer config names, if any.
    pub fn eos_text(&self) -> Option<&str> {
        self.eos_text.as_deref()
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, reason = "assertions in tests")]

    use super::*;

    fn message(role: &str, content: &str) -> ChatMessage {
        ChatMessage {
            role: role.to_string(),
            content: content.to_string(),
        }
    }

    #[test]
    fn a_template_renders_messages_and_the_generation_prompt() {
        let config = r#"{
            "chat_template": "{% for m in messages %}<|im_start|>{{ m.role }}\n{{ m.content }}<|im_end|>\n{% endfor %}{% if add_generation_prompt %}<|im_start|>assistant\n{% endif %}",
            "eos_token": "<|im_end|>"
        }"#;
        let template = ChatTemplate::from_tokenizer_config(config).unwrap();
        let text = template
            .render(&[message("system", "Be brief."), message("user", "Hi")])
            .unwrap();
        assert_eq!(
            text,
            "<|im_start|>system\nBe brief.<|im_end|>\n<|im_start|>user\nHi<|im_end|>\n<|im_start|>assistant\n"
        );
        assert_eq!(template.eos_text(), Some("<|im_end|>"));
    }

    #[test]
    fn python_string_methods_and_raise_exception_are_available() {
        let config = r#"{
            "chat_template": "{% if messages[0].content.startswith('x') %}{{ raise_exception('no x') }}{% endif %}{{ messages[0].content.strip() }}",
            "eos_token": {"content": "</s>"}
        }"#;
        let template = ChatTemplate::from_tokenizer_config(config).unwrap();
        assert_eq!(template.render(&[message("user", "  ok ")]).unwrap(), "ok");
        let error = template.render(&[message("user", "xy")]).unwrap_err();
        assert!(error.to_string().contains("no x"), "{error}");
    }

    #[test]
    fn a_config_without_a_template_is_refused() {
        assert!(ChatTemplate::from_tokenizer_config(r#"{"eos_token": "</s>"}"#).is_err());
    }
}
