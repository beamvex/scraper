pub(super) fn merge_token(token: &mut super::XToken, refreshed: super::XToken) {
    token.access_token = refreshed.access_token;
    token.refresh_token = refreshed.refresh_token.or(token.refresh_token.take());
    token.expires_in = refreshed.expires_in.or(token.expires_in);
    token.scope = refreshed.scope.or(token.scope.take());
    token.token_type = refreshed.token_type.or(token.token_type.take());
    token.obtained_at = Some(super::now_epoch_secs::now_epoch_secs());
}
