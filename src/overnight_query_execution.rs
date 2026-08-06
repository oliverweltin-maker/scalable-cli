use anyhow::Result;
use serde_json::{Value, json};

use crate::active_session::load_active_session;
use crate::config::{AppConfig, EnvConfig, TargetEnv};
use crate::dpop::DpopRuntimeOptions;
use crate::graphql::{GraphqlAccessContext, execute_graphql};
use crate::overnight_projections::{
    project_overnight_discovery_response, project_overnight_summary_response,
    project_overnight_transactions_response,
};
use crate::overnight_queries::{
    DISCOVER_OVERNIGHT_ACCOUNTS_QUERY, OVERNIGHT_SUMMARY_QUERY, OVERNIGHT_TRANSACTIONS_QUERY,
    OvernightSummaryInput, OvernightTransactionsInput, OvernightTransactionsOptions,
    overnight_discovery_variables, overnight_summary_variables, overnight_transactions_variables,
};
use crate::overnight_shared::{ResolvedOvernightSelection, resolve_overnight_selection};
use crate::payload_fingerprint::{checksum_for_payload, input_fingerprint_payload};
use crate::resolve_active_env;
use crate::session::{Session, SessionManager};
use crate::session_refresh::execute_with_refresh_retry;

pub(crate) fn execute_overnight_summary(
    args: crate::cli::OvernightArgs,
    config: &AppConfig,
    session_manager: &mut SessionManager,
) -> Result<Value> {
    let dpop_options = crate::channel::current_dpop_runtime_options(config);
    let dpop_options = &dpop_options;
    let env = resolve_active_env(session_manager)?;
    let env_cfg = crate::channel::current_env_config();
    let loaded = load_active_session(session_manager, env, &env_cfg, dpop_options)?;
    let mut session = loaded.session;
    let access_context = loaded.access_context;

    let selection = resolve_selection(
        session_manager,
        env,
        &env_cfg,
        &mut session,
        access_context,
        dpop_options,
        args.savings_account_id.as_deref(),
    )?;

    let input = OvernightSummaryInput::new(&session.person_id, &selection.savings_account_id)?;
    let summary_variables = overnight_summary_variables(&input)?;
    let summary_response = execute_with_refresh_retry(
        session_manager,
        env,
        &env_cfg,
        &mut session,
        dpop_options,
        |token| {
            execute_graphql(
                &env_cfg.graphql_url,
                token,
                OVERNIGHT_SUMMARY_QUERY,
                &summary_variables,
                Some("OvernightSummary"),
                access_context,
                dpop_options,
            )
        },
    )?;
    let result = project_overnight_summary_response(&input, &summary_response)?;

    Ok(json!({
        "savings_account_id": input.savings_account_id(),
        "selection": {
            "account": selection.selection_source,
        },
        "account": {
            "display_name": selection.display_name,
            "owner_kind": selection.owner_kind.as_str(),
            "is_active": selection.is_active,
        },
        "result": result,
    }))
}

pub(crate) fn execute_overnight_transactions(
    args: crate::cli::OvernightTransactionsArgs,
    config: &AppConfig,
    session_manager: &mut SessionManager,
) -> Result<Value> {
    let options = OvernightTransactionsOptions::new(
        args.page_size,
        args.cursor.as_deref(),
        &args.type_filter,
        args.search_term.as_deref(),
        args.from_time.as_deref(),
        args.to_time.as_deref(),
    )?;
    let dpop_options = crate::channel::current_dpop_runtime_options(config);
    let dpop_options = &dpop_options;
    let env = resolve_active_env(session_manager)?;
    let env_cfg = crate::channel::current_env_config();
    let loaded = load_active_session(session_manager, env, &env_cfg, dpop_options)?;
    let mut session = loaded.session;
    let access_context = loaded.access_context;
    let selection = resolve_selection(
        session_manager,
        env,
        &env_cfg,
        &mut session,
        access_context,
        dpop_options,
        args.savings_account_id.as_deref(),
    )?;
    let input = OvernightTransactionsInput::new(
        &session.person_id,
        &selection.savings_account_id,
        options,
    )?;
    let variables = overnight_transactions_variables(&input);
    let response = execute_with_refresh_retry(
        session_manager,
        env,
        &env_cfg,
        &mut session,
        dpop_options,
        |token| {
            execute_graphql(
                &env_cfg.graphql_url,
                token,
                OVERNIGHT_TRANSACTIONS_QUERY,
                &variables,
                Some("OvernightTransactions"),
                access_context,
                dpop_options,
            )
        },
    )?;
    let mut result = project_overnight_transactions_response(&input, &response)?;
    let normalized_input = variables.get("input").cloned().unwrap_or(Value::Null);
    let input_fingerprint = checksum_for_payload(&input_fingerprint_payload(&normalized_input));
    if let Some(result) = result.as_object_mut() {
        result.insert("input".to_string(), normalized_input);
        result.insert(
            "input_fingerprint".to_string(),
            Value::String(input_fingerprint),
        );
    }

    Ok(json!({
        "savings_account_id": input.savings_account_id(),
        "selection": {
            "account": selection.selection_source,
        },
        "account": {
            "display_name": selection.display_name,
            "owner_kind": selection.owner_kind.as_str(),
            "is_active": selection.is_active,
        },
        "result": result,
    }))
}

#[allow(clippy::too_many_arguments)]
fn resolve_selection(
    session_manager: &mut SessionManager,
    env: TargetEnv,
    env_cfg: &EnvConfig,
    session: &mut Session,
    access_context: GraphqlAccessContext,
    dpop_options: &DpopRuntimeOptions,
    explicit_savings_account_id: Option<&str>,
) -> Result<ResolvedOvernightSelection> {
    let discovery_variables = overnight_discovery_variables(&session.person_id)?;
    let discovery_response = execute_with_refresh_retry(
        session_manager,
        env,
        env_cfg,
        session,
        dpop_options,
        |token| {
            execute_graphql(
                &env_cfg.graphql_url,
                token,
                DISCOVER_OVERNIGHT_ACCOUNTS_QUERY,
                &discovery_variables,
                Some("DiscoverOvernightAccounts"),
                access_context,
                dpop_options,
            )
        },
    )?;
    let discovered_accounts = project_overnight_discovery_response(&discovery_response)?;
    resolve_overnight_selection(&discovered_accounts, explicit_savings_account_id)
}
