use super::*;
use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use http_body_util::BodyExt;
use serde_json::Value;
use tower::ServiceExt;

fn test_server() -> WorkbenchServer {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("repo root should exist");
    WorkbenchServer::new(WorkbenchLaunchConfig {
        workspace_root: root.clone(),
        spec_root: root.join("docs/syu"),
        bind: "127.0.0.1".to_string(),
        port: 3000,
        allow_remote_bind: false,
        show_log: false,
    })
    .expect("server should initialize")
}

async fn json_response(router: Router, request: Request<Body>) -> (StatusCode, Value) {
    let response = router
        .oneshot(request)
        .await
        .expect("request should succeed");
    let status = response.status();
    let bytes = BodyExt::collect(response.into_body())
        .await
        .expect("body should collect")
        .to_bytes();
    let json = serde_json::from_slice(&bytes).expect("json should parse");
    (status, json)
}

async fn text_response(router: Router, request: Request<Body>) -> (StatusCode, String) {
    let response = router
        .oneshot(request)
        .await
        .expect("request should succeed");
    let status = response.status();
    let bytes = BodyExt::collect(response.into_body())
        .await
        .expect("body should collect")
        .to_bytes();
    let text = String::from_utf8(bytes.to_vec()).expect("body should be utf8");
    (status, text)
}

#[tokio::test]
async fn index_route_renders_workbench_browser_entrypoint_and_css_asset() {
    let server = test_server();
    let (status, html) = text_response(
        server.router(),
        Request::builder()
            .uri("/")
            .body(Body::empty())
            .expect("request"),
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert!(html.contains("Syu Workbench"));
    assert!(html.contains("/assets/tailwind.css"));
    assert!(html.contains("<base href=\"/\">"));
    assert!(html.contains("syu-workbench-root"));
    assert!(html.contains("Syu"));
    assert!(html.contains("navigation"));
    assert!(html.contains(">Items</span>"));
    assert!(html.contains(">Diagnostics</span>"));
    assert!(html.contains("Type a command"));
    assert!(html.contains("data-command-palette"));
    assert!(html.contains("const rootSelector = '#syu-workbench-root'"));
    assert!(html.contains("currentRoot.replaceWith(nextRoot)"));
    assert!(html.contains("history.pushState"));
    assert!(html.contains("window.addEventListener('popstate'"));
    assert!(html.contains("fetch(url"));
    assert!(html.contains("initWorkbench(nextRoot)"));
    assert!(html.contains("data-scroll-key"));
    assert!(html.contains("scrollPositions"));
    assert!(html.contains("data-item-edit-toggle"));
    assert!(html.contains("form.dataset.enhanced = 'true'"));
    assert!(html.contains("if (!form.checkValidity()) return"));
    assert!(html.contains("form.dataset.running === 'true'"));
    assert!(html.contains("button.disabled = true"));
    assert!(html.contains("animate-spin"));
    assert!(!html.contains("Browser/server mode exposes the local Workbench API"));
}

#[tokio::test]
async fn items_surface_opens_before_workspace_initialization() {
    let tempdir = tempfile::tempdir().expect("tempdir");
    let server = WorkbenchServer::new(WorkbenchLaunchConfig {
        workspace_root: tempdir.path().to_path_buf(),
        spec_root: tempdir.path().join("docs/syu"),
        bind: "127.0.0.1".to_string(),
        port: 3000,
        allow_remote_bind: false,
        show_log: false,
    })
    .expect("uninitialized workspace should open");
    let (status, html) = text_response(
        server.router(),
        Request::builder()
            .uri("/?pane=items")
            .body(Body::empty())
            .expect("request"),
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert!(html.contains("data-items-toolbar=\"true\""));
    assert!(html.contains("Initialize workspace"));
    assert!(html.contains("cli.init"));
    server
        .spawn_watcher()
        .expect("uninitialized workspace should be watchable");
}

#[tokio::test]
async fn initialized_items_surface_hides_workspace_initialization() {
    let server = test_server();
    let (status, html) = text_response(
        server.router(),
        Request::builder()
            .uri("/?pane=items&spec_kind=requirement")
            .body(Body::empty())
            .expect("request"),
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert!(!html.contains("data-initialize-workspace=\"true\""));
    assert!(html.contains("data-new-spec-item=\"requirement\""));
    assert!(html.contains("+ New requirement"));
}

#[tokio::test]
async fn cli_information_command_renders_spec_browser_without_running_cli() {
    let server = test_server();
    let (status, html) = text_response(
        server.router(),
        Request::builder()
            .uri("/?pane=commands&cli=cli.list&query=requirements")
            .body(Body::empty())
            .expect("request"),
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert!(html.contains("Spec tree"));
    assert!(html.contains("Search specs"));
    assert!(html.contains("data-category-layout=\"browse\""));
    assert!(html.contains("spec_item="));
    assert!(!html.contains("name=\"run\" value=\"1\""));
    assert!(!html.contains("stdout:"));
    assert!(!html.contains("stderr:"));
}

#[tokio::test]
async fn role_menu_routes_commands_to_items_and_diagnostics() {
    let server = test_server();
    let (_, items_html) = text_response(
        server.router(),
        Request::builder()
            .uri("/?pane=commands&cli=cli.show&spec_item=REQ-WORKBENCH-001")
            .body(Body::empty())
            .expect("request"),
    )
    .await;
    assert!(items_html.contains("aria-current=\"page\""));
    assert!(items_html.contains("data-items-toolbar=\"true\""));
    assert!(items_html.contains("data-item-editor=\"true\""));

    let (_, diagnostics_html) = text_response(
        server.router(),
        Request::builder()
            .uri("/?pane=diagnostics")
            .body(Body::empty())
            .expect("request"),
    )
    .await;
    assert!(diagnostics_html.contains("data-diagnostics-overview=\"true\""));
    assert!(diagnostics_html.contains("data-diagnostic-tool=\"validate\""));
    assert!(diagnostics_html.contains("data-diagnostic-tool=\"doctor\""));
    assert!(diagnostics_html.contains("data-diagnostic-tool=\"audit\""));
    assert!(diagnostics_html.contains("data-diagnostic-tool=\"goal\""));
}

#[tokio::test]
async fn explicit_navigation_panes_drop_stale_cli_selection() {
    let server = test_server();
    let router = server.router();
    let stale = "cli=cli.show&spec_item=REQ-WORKBENCH-001";

    let (_, items_html) = text_response(
        router.clone(),
        Request::builder()
            .uri(format!("/?pane=items&lang=en&{stale}"))
            .body(Body::empty())
            .expect("request"),
    )
    .await;
    assert!(items_html.contains("href=\"?pane=request&#38;lang=en\""));
    assert!(!items_html.contains("href=\"?pane=request&#38;lang=en&#38;cli=cli.show"));
    assert!(!items_html.contains("sidebar="));
    assert!(!items_html.contains("href=\"?pane=pulse"));

    let (_, work_html) = text_response(
        router.clone(),
        Request::builder()
            .uri(format!("/?pane=pulse&lang=en&{stale}"))
            .body(Body::empty())
            .expect("request"),
    )
    .await;
    assert!(work_html.contains(">Request intake</h1>"));
    assert!(work_html.contains("data-role-subviews=\"true\""));
    assert!(!work_html.contains("data-items-toolbar=\"true\""));
    assert!(!work_html.contains(">Work</h1>"));

    let (_, work_alias_html) = text_response(
        router.clone(),
        Request::builder()
            .uri(format!("/?pane=work&lang=en&{stale}"))
            .body(Body::empty())
            .expect("request"),
    )
    .await;
    assert!(work_alias_html.contains(">Request intake</h1>"));
    assert!(work_alias_html.contains("data-role-subviews=\"true\""));
    assert!(!work_alias_html.contains("data-items-toolbar=\"true\""));
    assert!(!work_alias_html.contains(">Work</h1>"));

    let (_, scope_html) = text_response(
        router.clone(),
        Request::builder()
            .uri(format!("/?pane=branch&lang=en&{stale}"))
            .body(Body::empty())
            .expect("request"),
    )
    .await;
    assert!(scope_html.contains(">Scope</h1>"));
    assert!(scope_html.contains("data-scope-overview=\"true\""));
    assert!(!scope_html.contains("data-items-toolbar=\"true\""));

    let (_, diagnostics_html) = text_response(
        router.clone(),
        Request::builder()
            .uri(format!("/?pane=diagnostics&lang=en&{stale}"))
            .body(Body::empty())
            .expect("request"),
    )
    .await;
    assert!(diagnostics_html.contains(">Diagnostics</h1>"));
    assert!(diagnostics_html.contains("data-diagnostics-overview=\"true\""));
    assert!(!diagnostics_html.contains("data-items-toolbar=\"true\""));

    let (_, palette_html) = text_response(
        router,
        Request::builder()
            .uri(format!("/?pane=commands&lang=en&{stale}"))
            .body(Body::empty())
            .expect("request"),
    )
    .await;
    assert!(palette_html.contains("data-items-toolbar=\"true\""));
}

#[tokio::test]
async fn diagnostics_refresh_all_runs_unique_tools_and_skips_missing_goal() {
    let server = test_server();
    let (status, html) = text_response(
        server.router(),
        Request::builder()
            .method("POST")
            .uri("/run")
            .header("content-type", "application/x-www-form-urlencoded")
            .header("host", "localhost:3000")
            .header("origin", "http://localhost:3000")
            .body(Body::from("pane=diagnostics&diagnostics_all=1"))
            .expect("request"),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert!(html.contains("All diagnostics refreshed"));
    assert!(html.contains("Workspace validation"));
    assert!(html.contains("Contributor doctor"));
    assert!(html.contains("Specification audit"));
    assert!(html.contains("Goal check"));
    assert!(html.contains("Skipped because no active Goal Plan is available."));
}

#[tokio::test]
async fn item_edit_requires_review_before_writing_source() {
    let server = test_server();
    let source = server
        .inner
        .config
        .spec_root
        .join("requirements/core/workbench.yaml");
    let before = fs::read_to_string(&source).expect("source");
    let (status, html) = text_response(
        server.router(),
        Request::builder()
            .method("POST")
            .uri("/run")
            .header("content-type", "application/x-www-form-urlencoded")
            .header("host", "localhost:3000")
            .header("origin", "http://localhost:3000")
            .body(Body::from(
                "pane=items&item_edit=REQ-WORKBENCH-001&title=Preview+title&description=Preview+body&priority=medium&status=implemented&linked_policies=POL-005",
            ))
            .expect("request"),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert!(html.contains("data-item-edit-preview=\"true\""));
    assert!(html.contains("Review this source-preserving item diff before applying it."));
    assert!(html.contains("Apply reviewed change"));
    assert!(html.contains("name=\"spec_item\" value=\"REQ-WORKBENCH-001\""));
    assert_eq!(
        fs::read_to_string(source).expect("source after preview"),
        before
    );
}

#[tokio::test]
async fn item_edit_previews_and_applies_reciprocal_links_together() {
    let tempdir = tempfile::tempdir().expect("tempdir");
    let spec_root = tempdir.path().join("docs/syu");
    let requirements = spec_root.join("requirements/core");
    let features = spec_root.join("features/core");
    fs::create_dir_all(&requirements).expect("requirements dir");
    fs::create_dir_all(&features).expect("features dir");
    let requirement_path = requirements.join("requirements.yaml");
    let feature_path = features.join("core.yaml");
    let requirement_before = "category: Core\nprefix: REQ\nrequirements:\n  - id: REQ-1\n    title: Requirement\n    description: Keep links reciprocal.\n    priority: medium\n    status: planned\n    linked_features:\n      - FEAT-1\n    tests: {}\n";
    let feature_before = "category: Core\nversion: 1\nfeatures:\n  - id: FEAT-1\n    title: First\n    summary: First feature.\n    status: planned\n    linked_requirements:\n      - REQ-1\n    implementations: {}\n  - id: FEAT-2\n    title: Second\n    summary: Second feature.\n    status: planned\n    linked_requirements: []\n    implementations: {}\n";
    fs::write(&requirement_path, requirement_before).expect("requirement");
    fs::write(&feature_path, feature_before).expect("features");
    let server = WorkbenchServer::new(WorkbenchLaunchConfig {
        workspace_root: tempdir.path().to_path_buf(),
        spec_root: spec_root.clone(),
        bind: "127.0.0.1".to_string(),
        port: 3000,
        allow_remote_bind: false,
        show_log: false,
    })
    .expect("server");
    let values: ItemEditValues = serde_json::from_value(serde_json::json!({
        "title": "Requirement",
        "summary": "",
        "description": "Keep links reciprocal.",
        "product_design_principle": "",
        "coding_guideline": "",
        "priority": "medium",
        "status": "planned",
        "linked_philosophies": [],
        "linked_policies": [],
        "linked_requirements": [],
        "linked_features": ["FEAT-2"],
        "tests_yaml": "",
        "implementations_yaml": "",
        "source_hashes": {}
    }))
    .expect("edit values");

    let preview = preview_or_apply_item_edit(&server, "REQ-1", values, false)
        .await
        .expect("preview");
    assert!(!preview.applied);
    assert!(preview.diff.contains("FEAT-1"));
    assert!(preview.diff.contains("FEAT-2"));
    assert_eq!(
        fs::read_to_string(&requirement_path).expect("requirement after preview"),
        requirement_before
    );
    assert_eq!(
        fs::read_to_string(&feature_path).expect("features after preview"),
        feature_before
    );

    let reviewed: ItemEditValues =
        serde_json::from_str(&preview.apply_payload).expect("review payload");
    fs::write(&feature_path, format!("{feature_before}\n")).expect("external feature edit");
    let stale_error = preview_or_apply_item_edit(&server, "REQ-1", reviewed.clone(), true)
        .await
        .expect_err("stale reciprocal source should be rejected");
    assert!(stale_error.to_string().contains("changed after preview"));
    assert_eq!(
        fs::read_to_string(&requirement_path).expect("requirement after rejected apply"),
        requirement_before
    );
    fs::write(&feature_path, feature_before).expect("restore features");

    let applied = preview_or_apply_item_edit(&server, "REQ-1", reviewed, true)
        .await
        .expect("apply");
    assert!(applied.applied);
    let requirement_after = fs::read_to_string(requirement_path).expect("requirement after apply");
    let feature_after = fs::read_to_string(feature_path).expect("features after apply");
    assert!(requirement_after.contains("FEAT-2"));
    assert!(!requirement_after.contains("FEAT-1"));
    let first = feature_after
        .split("- id: FEAT-2")
        .next()
        .expect("first feature block");
    let second = feature_after
        .split("- id: FEAT-2")
        .nth(1)
        .expect("second feature block");
    assert!(!first.contains("REQ-1"));
    assert!(second.contains("REQ-1"));
}

#[test]
fn item_block_replacement_preserves_other_items_and_unknown_fields() {
    let raw = "category: Core\nrequirements:\n  - id: REQ-1\n    title: Before\n    unknown: keep\n  - id: REQ-2\n    title: Other\n";
    let item: serde_yaml::Value =
        serde_yaml::from_str("id: REQ-1\ntitle: After\nunknown: keep\n").expect("item");
    let updated = replace_yaml_item_block(raw, "REQ-1", &item).expect("replace");
    assert!(updated.contains("title: After"));
    assert!(updated.contains("unknown: keep"));
    assert!(updated.contains("- id: REQ-2\n    title: Other"));
}

#[tokio::test]
async fn command_palette_and_spec_search_queries_are_independent() {
    let server = test_server();
    let (status, html) = text_response(
        server.router(),
        Request::builder()
            .uri("/?pane=commands&cli=cli.show&query=show&spec_query=repository")
            .body(Body::empty())
            .expect("request"),
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert!(html.contains("name=\"query\" value=\"show\""));
    assert!(html.contains("name=\"spec_query\" value=\"repository\""));
    assert!(html.contains("spec_query=repository"));
}

#[tokio::test]
async fn category_filter_and_typed_check_result_render_from_browser_route() {
    let server = test_server();
    let router = server.router();
    let (_, filtered_html) = text_response(
        router.clone(),
        Request::builder()
            .uri("/?pane=commands&category=check")
            .body(Body::empty())
            .expect("request"),
    )
    .await;

    assert!(filtered_html.contains("data-command-id=\"cli.validate\""));
    assert!(!filtered_html.contains("data-command-id=\"cli.init\""));

    let (_, get_html) = text_response(
        router,
        Request::builder()
            .uri("/?pane=commands&category=check&cli=cli.validate&run=1")
            .body(Body::empty())
            .expect("request"),
    )
    .await;

    assert!(get_html.contains("syu validate ."));

    let (_, result_html) = text_response(
        server.router(),
        Request::builder()
            .method("POST")
            .uri("/run")
            .header("content-type", "application/x-www-form-urlencoded")
            .header("host", "localhost:3000")
            .header("origin", "http://localhost:3000")
            .body(Body::from(
                "pane=commands&category=check&cli=cli.validate&run=1",
            ))
            .expect("request"),
    )
    .await;

    assert!(result_html.contains("data-result-kind=\"CheckDetail\""));
    assert!(result_html.contains("data-check-summary=\"true\""));
}

#[test]
fn structured_results_use_short_category_summaries_instead_of_json_headings() {
    let result = typed_result_from_json(
        Locale::En,
        CommandCategory::Check,
        r#"{"issues":[{"severity":"error","message":"broken link"}]}"#.to_string(),
        CommandResultStatus::Fail,
        serde_json::json!({
            "issues": [{"severity": "error", "message": "broken link"}]
        }),
    );

    assert_eq!(result.summary, "1 checks · fail");
    assert_eq!(result.items[0].title, "broken link");
    assert!(!result.summary.contains('{'));
}

#[test]
fn structured_object_results_split_into_readable_field_items() {
    let result = typed_result_from_json(
        Locale::En,
        CommandCategory::Check,
        String::new(),
        CommandResultStatus::Pass,
        serde_json::json!({
            "workspace_root": "/workspace",
            "definition_counts": {"requirements": 4},
            "issues": []
        }),
    );

    assert_eq!(result.summary, "2 checks · pass");
    assert_eq!(result.items[0].title, "Definition counts");
    assert_eq!(result.items[1].title, "Workspace root");
    assert!(result.items.iter().all(|item| item.detail.len() < 80));
}

#[tokio::test]
async fn css_route_serves_the_shared_tailwind_asset() {
    let server = test_server();
    let (status, css) = text_response(
        server.router(),
        Request::builder()
            .uri("/assets/tailwind.css")
            .body(Body::empty())
            .expect("request"),
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert!(css.contains("--color-command-active"));
}

#[tokio::test]
async fn server_smoke_covers_root_css_health_and_actions() {
    let server = test_server();
    let router = server.router();

    let (root_status, root_html) = text_response(
        router.clone(),
        Request::builder()
            .uri("/")
            .body(Body::empty())
            .expect("request"),
    )
    .await;
    let (css_status, css) = text_response(
        router.clone(),
        Request::builder()
            .uri("/assets/tailwind.css")
            .body(Body::empty())
            .expect("request"),
    )
    .await;
    let (health_status, health) = json_response(
        router.clone(),
        Request::builder()
            .uri("/api/health")
            .body(Body::empty())
            .expect("request"),
    )
    .await;
    let (actions_status, actions) = json_response(
        router,
        Request::builder()
            .uri("/api/actions")
            .body(Body::empty())
            .expect("request"),
    )
    .await;

    assert_eq!(root_status, StatusCode::OK);
    assert!(root_html.contains("Syu"));
    assert!(root_html.contains("Type a command"));
    assert!(root_html.contains("navigation"));
    assert!(root_html.contains(">Items</span>"));
    assert!(root_html.contains(">Diagnostics</span>"));
    assert!(root_html.contains("data-command-palette"));
    assert_eq!(css_status, StatusCode::OK);
    assert!(css.contains("--color-background"));
    assert_eq!(health_status, StatusCode::OK);
    assert_eq!(health["ok"], true);
    assert_eq!(actions_status, StatusCode::OK);
    assert!(actions["actions"].as_array().is_some_and(|actions| {
        actions
            .iter()
            .any(|action| action["id"] == "request.classify")
    }));
}

#[tokio::test]
async fn every_palette_action_can_be_selected_and_submitted_from_browser_route() {
    let server = test_server();
    let router = server.router();
    let (_, palette_html) = text_response(
        router.clone(),
        Request::builder()
            .uri("/?pane=commands")
            .body(Body::empty())
            .expect("request"),
    )
    .await;

    for action in shared_workbench::WorkbenchActionRegistry::standard().actions() {
        let action_id = action.id.label();
        assert!(
            palette_html.contains(&format!("data-command-id=\"{action_id}\"")),
            "{action_id} should be visible in the command palette"
        );

        let (select_status, selected_html) = text_response(
            router.clone(),
            Request::builder()
                .uri(format!("/?pane=commands&action={action_id}"))
                .body(Body::empty())
                .expect("request"),
        )
        .await;
        assert_eq!(select_status, StatusCode::OK, "{action_id} should select");
        assert!(
            selected_html.contains(&format!("name=\"action\" value=\"{action_id}\"")),
            "{action_id} should render a runnable action form"
        );

        let (run_status, run_html) = text_response(
            router.clone(),
            Request::builder()
                .method("POST")
                .uri("/run")
                .header("content-type", "application/x-www-form-urlencoded")
                .header("host", "localhost:3000")
                .header("origin", "http://localhost:3000")
                .body(Body::from(format!(
                    "pane=commands&action={action_id}&run=1&action_input=Workbench&action_confirm=1"
                )))
                .expect("request"),
        )
        .await;
        assert_eq!(run_status, StatusCode::OK, "{action_id} should submit");
        assert!(
            run_html.contains(&action_id.replace('.', " "))
                || run_html.contains("failed to run")
                || run_html.contains("input required")
                || run_html.contains("data-result-kind="),
            "{action_id} should render a submission result"
        );
        assert!(
            run_html.contains(&format!(
                "data-category-layout=\"{}\"",
                syu_app_ui::model::workbench_action_category(action.id).slug()
            )),
            "{action_id} should render its category-specific result layout"
        );
    }
}

#[tokio::test]
async fn every_palette_cli_command_can_be_selected_and_submitted_from_browser_route() {
    let server = test_server();
    let router = server.router();
    let (_, palette_html) = text_response(
        router.clone(),
        Request::builder()
            .uri("/?pane=commands")
            .body(Body::empty())
            .expect("request"),
    )
    .await;

    for command in cli_command_catalog(syu_app_ui::Locale::En) {
        assert!(
            palette_html.contains(&format!("data-command-id=\"{}\"", command.id)),
            "{} should be visible in the command palette",
            command.id
        );

        let (select_status, selected_html) = text_response(
            router.clone(),
            Request::builder()
                .uri(format!("/?pane=commands&cli={}", command.id))
                .body(Body::empty())
                .expect("request"),
        )
        .await;
        assert_eq!(
            select_status,
            StatusCode::OK,
            "{} should select",
            command.id
        );
        assert!(
            selected_html.contains(&format!("name=\"cli\" value=\"{}\"", command.id)),
            "{} should render a runnable CLI form",
            command.id
        );

        let (run_status, run_html) = text_response(
            router.clone(),
            Request::builder()
                .method("POST")
                .uri("/run")
                .header("content-type", "application/x-www-form-urlencoded")
                .header("host", "localhost:3000")
                .header("origin", "http://localhost:3000")
                .body(Body::from(format!(
                    "pane=commands&cli={}&run=1",
                    command.id
                )))
                .expect("request"),
        )
        .await;
        assert_eq!(run_status, StatusCode::OK, "{} should submit", command.id);
        assert!(
            run_html.contains(command.invocation)
                || run_html.contains("needs input before it can run")
                || run_html.contains("needs confirmation before writing files")
                || (command.category() == CommandCategory::Browse
                    && command.opens_spec_browser
                    && run_html.contains("data-category-layout=\"browse\"")),
            "{} should render a submission result",
            command.id
        );
        assert!(
            run_html.contains(&format!(
                "data-category-layout=\"{}\"",
                command.category().slug()
            )),
            "{} should render its category-specific result layout",
            command.id
        );
    }
}

#[test]
fn cli_task_defaults_prepare_readable_fixtures() {
    let tempdir = tempfile::tempdir().expect("tempdir");
    let workspace_root = tempdir.path();

    ensure_cli_task_fixture(
        "cli.task.classify",
        workspace_root,
        "target/syu/workbench/request.yaml",
    )
    .expect("request fixture");
    ensure_cli_task_fixture(
        "cli.task.check",
        workspace_root,
        "target/syu/workbench/goal.yaml",
    )
    .expect("goal fixture");

    let request_path = workspace_root.join("target/syu/workbench/request.yaml");
    let goal_path = workspace_root.join("target/syu/workbench/goal.yaml");
    let request = fs::read_to_string(request_path).expect("request fixture should be readable");
    let goal = fs::read_to_string(goal_path).expect("goal fixture should be readable");

    assert!(request.contains("REQ-WORKBENCH-001"));
    assert!(goal.contains("GOAL-WORKBENCH-PALETTE-001"));
    assert!(goal.contains("**"));
}

#[test]
fn cli_task_fixtures_reject_paths_outside_workbench_target() {
    let tempdir = tempfile::tempdir().expect("tempdir");

    for path in [
        "../request.yaml",
        "target/syu/other/request.yaml",
        "/tmp/request.yaml",
    ] {
        assert!(
            ensure_cli_task_fixture("cli.task.classify", tempdir.path(), path).is_err(),
            "{path} should be rejected"
        );
    }
}

#[tokio::test]
async fn command_run_rejects_cross_origin_posts() {
    let server = test_server();
    let response = server
        .router()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/run")
                .header("content-type", "application/x-www-form-urlencoded")
                .header("host", "localhost:3000")
                .header("origin", "https://attacker.example")
                .body(Body::from("cli=cli.validate&run=1"))
                .expect("request"),
        )
        .await
        .expect("response");

    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}

#[test]
fn cli_task_check_preview_passes_required_range_argument() {
    let args = cli_command_args("cli.task.check", "target/syu/workbench/goal.yaml")
        .expect("task check args");

    assert_eq!(
        args,
        vec![
            "task".to_string(),
            "check".to_string(),
            "target/syu/workbench/goal.yaml".to_string(),
            "--range".to_string(),
            "origin/main...HEAD".to_string(),
            "--format".to_string(),
            "json".to_string(),
        ]
    );
}

#[tokio::test]
async fn every_palette_cli_command_has_a_preview_and_argument_path() {
    let tempdir = tempfile::tempdir().expect("tempdir");

    for command in cli_command_catalog(syu_app_ui::Locale::En) {
        let cli_arg = match command.id {
            "cli.show" | "cli.log" => "REQ-WORKBENCH-001",
            "cli.search" => "workbench",
            "cli.explain" | "cli.relate" => "docs/syu/requirements.md",
            "cli.trace" => "crates/syu-workbench-server/src/lib.rs",
            "cli.completion" => "zsh",
            "cli.task.classify" | "cli.task.scope" | "cli.task.scaffold" | "cli.task.plan" => {
                "target/syu/workbench/request.yaml"
            }
            "cli.task.test_select" | "cli.task.check" => "target/syu/workbench/goal.yaml",
            "cli.add" => "requirement REQ-WORKBENCH-PLAYWRIGHT-001",
            _ => "",
        };

        let resolved_arg = cli_default_arg(command.id, cli_arg);
        assert!(
            cli_command_args(command.id, resolved_arg).is_some(),
            "{} should resolve to CLI arguments",
            command.id
        );

        let preview = run_cli_command_preview(
            command.id,
            tempdir.path(),
            Some(cli_arg),
            false,
            false,
            syu_app_ui::Locale::En,
        )
        .await
        .unwrap_or_else(|| panic!("{} should produce a preview", command.id));
        assert_eq!(preview.id, command.id);
        assert!(!preview.result_summary.trim().is_empty());
    }
}

#[tokio::test]
async fn cli_preview_can_include_stdout_and_stderr_log_sections() {
    let tempdir = tempfile::tempdir().expect("tempdir");
    let preview = run_cli_command_preview(
        "cli.templates",
        tempdir.path(),
        None,
        false,
        true,
        syu_app_ui::Locale::En,
    )
    .await
    .expect("templates preview");

    assert!(preview.result_summary.contains("stdout:"));
    assert!(preview.result_summary.contains("stderr:"));
}

#[tokio::test]
async fn cli_preview_hides_diagnostics_without_show_log() {
    let tempdir = tempfile::tempdir().expect("tempdir");
    let preview = run_cli_command_preview(
        "cli.templates",
        tempdir.path(),
        None,
        false,
        false,
        syu_app_ui::Locale::En,
    )
    .await
    .expect("templates preview");

    assert!(preview.result.diagnostics.is_none());
}

#[tokio::test]
async fn health_endpoint_reports_server_details() {
    let server = test_server();
    let (status, json) = json_response(
        server.router(),
        Request::builder()
            .uri("/api/health")
            .body(Body::empty())
            .expect("request"),
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(json["ok"], true);
    assert_eq!(json["bind"], "127.0.0.1");
    assert_eq!(json["port"], 3000);
}

#[tokio::test]
async fn actions_endpoint_lists_registry() {
    let server = test_server();
    let (status, json) = json_response(
        server.router(),
        Request::builder()
            .uri("/api/actions")
            .body(Body::empty())
            .expect("request"),
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert!(
        json["actions"]
            .as_array()
            .is_some_and(|actions| !actions.is_empty())
    );
    assert!(
        json["availability"]
            .as_array()
            .is_some_and(|availability| !availability.is_empty())
    );
}

#[tokio::test]
async fn request_plan_endpoint_returns_goal_plan() {
    let server = test_server();
    let body = serde_json::json!({
        "request": {
            "version": 1,
            "request": "Add Workbench planning coverage",
            "context": {
                "linked_ids": ["REQ-WORKBENCH-006"]
            }
        },
        "request_path": "request.yaml"
    });
    let (status, json) = json_response(
        server.router(),
        Request::builder()
            .method("POST")
            .uri("/api/request/plan")
            .header("content-type", "application/json")
            .body(Body::from(body.to_string()))
            .expect("request"),
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(json["kind"], "syu.goal_plan");
    assert_eq!(json["goal"]["id"], "GOAL-001");
}

#[tokio::test]
async fn branch_scope_endpoint_returns_report() {
    let server = test_server();
    let (status, json) = json_response(
        server.router(),
        Request::builder()
            .uri("/api/branch/scope?range=HEAD...HEAD")
            .body(Body::empty())
            .expect("request"),
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(json["range"], "HEAD...HEAD");
    assert!(json["changed_files"].as_array().is_some());

    let snapshot = server.inner.state.read().await.clone();
    assert_eq!(
        snapshot
            .branch_scope
            .as_ref()
            .map(|report| report.range.as_str()),
        Some("HEAD...HEAD")
    );
}

#[tokio::test]
async fn request_new_action_persists_active_request() {
    let server = test_server();
    let body = serde_json::json!({
        "version": 1,
        "request": "Create a new active request",
        "context": {
            "linked_ids": ["REQ-WORKBENCH-001"]
        }
    });
    let (status, json) = json_response(
        server.router(),
        Request::builder()
            .method("POST")
            .uri("/api/actions/request.new/run")
            .header("content-type", "application/json")
            .body(Body::from(body.to_string()))
            .expect("request"),
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        json["result"]["artifact"]["request"],
        "Create a new active request"
    );

    let snapshot = server.inner.state.read().await.clone();
    assert_eq!(
        snapshot
            .request
            .as_ref()
            .and_then(|request| request.artifact.as_ref())
            .map(|artifact| artifact.request.as_str()),
        Some("Create a new active request")
    );
}

#[tokio::test]
async fn goal_check_endpoint_returns_report() {
    let server = test_server();
    let plan = goal_plan_from_request(
        &server,
        &RequestPlanRequest {
            request: RequestArtifact {
                version: 1,
                request: "Keep goal checking typed".to_string(),
                context: Default::default(),
            },
            request_path: Some("request.yaml".to_string()),
        },
    )
    .await;
    let body = serde_json::json!({
        "plan": plan,
        "range": "HEAD...HEAD"
    });
    let (status, json) = json_response(
        server.router(),
        Request::builder()
            .method("POST")
            .uri("/api/goals/goal-1/check")
            .header("content-type", "application/json")
            .body(Body::from(body.to_string()))
            .expect("request"),
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(json["range"], "HEAD...HEAD");
    assert_eq!(json["plan_path"], "request.yaml");

    let snapshot = server.inner.state.read().await.clone();
    assert_eq!(snapshot.evidence_timeline.entries.len(), 1);
    assert_eq!(
        snapshot.evidence_timeline.entries[0].kind,
        WorkbenchEvidenceKind::GoalPlanCheckReport
    );
    assert_eq!(
        snapshot.evidence_timeline.entries[0].goal_id.as_deref(),
        Some("goal-1")
    );
    assert_eq!(
        snapshot.evidence_timeline.entries[0].status,
        EvidenceStatus::Pass
    );
}

#[tokio::test]
async fn goal_test_select_endpoint_records_evidence() {
    let server = test_server();
    let plan = goal_plan_from_request(
        &server,
        &RequestPlanRequest {
            request: RequestArtifact {
                version: 1,
                request: "Select tests for the goal".to_string(),
                context: Default::default(),
            },
            request_path: Some("request.yaml".to_string()),
        },
    )
    .await;
    let body = serde_json::to_string(&plan).expect("plan json");
    let (status, json) = json_response(
        server.router(),
        Request::builder()
            .method("POST")
            .uri("/api/goals/goal-1/test-select")
            .header("content-type", "application/json")
            .body(Body::from(body))
            .expect("request"),
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(json["goal_id"], "goal-1");

    let snapshot = server.inner.state.read().await.clone();
    assert_eq!(snapshot.evidence_timeline.entries.len(), 1);
    assert_eq!(
        snapshot.evidence_timeline.entries[0].kind,
        WorkbenchEvidenceKind::TaskTestSelectionPlan
    );
    assert_eq!(
        snapshot.evidence_timeline.entries[0].goal_id.as_deref(),
        Some("goal-1")
    );
    assert_eq!(
        snapshot.evidence_timeline.entries[0].status,
        EvidenceStatus::Pass
    );
}

#[tokio::test]
async fn validation_action_records_evidence() {
    let server = test_server();
    let (status, json) = json_response(
        server.router(),
        Request::builder()
            .method("POST")
            .uri("/api/actions/validation.run/run")
            .header("content-type", "application/json")
            .body(Body::from("{}"))
            .expect("request"),
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(json["result"]["status"], "ok");

    let snapshot = server.inner.state.read().await.clone();
    assert_eq!(snapshot.evidence_timeline.entries.len(), 1);
    assert_eq!(
        snapshot.evidence_timeline.entries[0].kind,
        WorkbenchEvidenceKind::ValidationReport
    );
    assert_eq!(
        snapshot.evidence_timeline.entries[0].status,
        EvidenceStatus::Pass
    );
}

#[tokio::test]
async fn events_endpoint_streams_initial_reload_event() {
    let server = test_server();
    let response = server
        .router()
        .oneshot(
            Request::builder()
                .uri("/api/events")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("request should succeed");

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response
            .headers()
            .get(axum::http::header::CONTENT_TYPE)
            .and_then(|value| value.to_str().ok()),
        Some("text/event-stream")
    );

    let mut body = response.into_body();
    let frame = body
        .frame()
        .await
        .expect("frame should exist")
        .expect("frame");
    let bytes = frame.into_data().expect("data frame");
    let text = std::str::from_utf8(&bytes).expect("utf8");
    assert!(text.contains("workspace_reloaded"));
}
