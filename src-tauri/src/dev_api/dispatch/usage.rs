use super::*;

macro_rules! command {
    ($future:expr) => { as_json($future.await) };
}

pub(super) async fn usage(app: &AppHandle, request: &InvokeRequest) -> Result<Option<Value>, CommandError> {
    let result = match request.command.as_str() {
        "test_usage_query" => command!(commands::test_usage_query(argument(
            &request.args,
            "request",
        )?)),
        "query_profile_usage" => command!(commands::query_profile_usage(
            app.clone(),
            argument(&request.args, "target")?,
            argument(&request.args, "profileId")?,
        )),
        "read_profile_usage" => command!(commands::read_profile_usage(
            app.clone(),
            argument(&request.args, "target")?,
            argument(&request.args, "profileId")?,
        )),
        "get_model_usage_report" => {
            command!(commands::model_usage::get_model_usage_report(
                app.clone(),
                argument::<ModelUsageRequest>(&request.args, "request")?,
            ))
        }
        "get_usage_history" => command!(commands::usage_history::get_usage_history(
            app.clone(),
            argument::<UsageHistoryRequest>(&request.args, "request")?,
        )),
        "list_runtime_logs" => command!(commands::runtime_log::list_runtime_logs(app.clone(),
            optional_argument(&request.args, "level")?)),
        "open_runtime_log_dir" => {
            command!(commands::runtime_log::open_runtime_log_dir(app.clone()))
        }
        _ => return Ok(None),
    };
    result.map(Some)
}
