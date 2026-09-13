//! Chat templates read from tokenizer_config.json and rendered.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "assertions in tests"
)]

use piramid_model::inference::tokenizer::{ChatMessage, ChatTemplate};

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
