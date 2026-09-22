//! `AnthropicAdapter::identity_from_bytes`: lê `oauthAccount.emailAddress` do
//! `.claude.json`, o tier (org → usuário como fallback) e preserva o cru.

use router_core::engine::anthropic_adapter::AnthropicAdapter;

#[test]
fn reads_email_org_and_org_tier() {
    let json = br#"{
        "oauthAccount": {
            "emailAddress": "conta1@exemplo.com",
            "organizationName": "Acme",
            "organizationRateLimitTier": "default_claude_max_20x",
            "userRateLimitTier": "default_claude_pro"
        },
        "hasCompletedOnboarding": true
    }"#;
    let id = AnthropicAdapter::identity_from_bytes(json).unwrap();
    assert_eq!(id.email, "conta1@exemplo.com");
    assert_eq!(id.organization_name.as_deref(), Some("Acme"));
    // O tier de ORG vence o de usuário.
    assert_eq!(
        id.rate_limit_tier.as_deref(),
        Some("default_claude_max_20x")
    );
    // O cru é preservado inteiro.
    assert!(id.raw.contains_key("organizationRateLimitTier"));
}

#[test]
fn falls_back_to_user_tier_without_org() {
    let json = br#"{"oauthAccount":{"emailAddress":"p@exemplo.com","userRateLimitTier":"default_claude_pro"}}"#;
    let id = AnthropicAdapter::identity_from_bytes(json).unwrap();
    assert_eq!(id.rate_limit_tier.as_deref(), Some("default_claude_pro"));
    assert_eq!(id.organization_name, None);
}

#[test]
fn no_oauth_account_is_none() {
    assert!(AnthropicAdapter::identity_from_bytes(br#"{"hasCompletedOnboarding":true}"#).is_none());
    assert!(AnthropicAdapter::identity_from_bytes(br#"{"oauthAccount":{}}"#).is_none());
    assert!(AnthropicAdapter::identity_from_bytes(b"not json").is_none());
}
