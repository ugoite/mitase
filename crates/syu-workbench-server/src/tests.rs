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
        .expect("body")
        .to_bytes();
    (status, serde_json::from_slice(&bytes).expect("json"))
}

async fn text_response(router: Router, request: Request<Body>) -> (StatusCode, String) {
    let response = router
        .oneshot(request)
        .await
        .expect("request should succeed");
    let status = response.status();
    let bytes = BodyExt::collect(response.into_body())
        .await
        .expect("body")
        .to_bytes();
    (status, String::from_utf8(bytes.to_vec()).expect("utf8"))
}

#[tokio::test]
async fn index_renders_new_page_contract_and_navigation_script() {
    let (status, html) = text_response(
        test_server().router(),
        Request::builder()
            .uri("/")
            .body(Body::empty())
            .expect("request"),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert!(html.contains(">Work</span>"));
    assert!(html.contains(">Scope</span>"));
    assert!(html.contains(">Items</span>"));
    assert!(html.contains(">Diagnostics</span>"));
    assert!(html.contains("Search commands, tasks, and Items"));
    assert!(html.contains("data-command-palette"));
    assert!(html.contains("new EventSource('/api/events')"));
    assert!(html.contains("/api/diagnostics/run"));
    assert!(html.contains("[data-work-selector]"));
    assert!(html.contains("[data-diagnostics-filter]"));
    assert!(html.contains("[data-scope-mode]"));
    assert!(html.contains("/api/actions/branch.infer_goal/run"));
    assert!(html.contains("data.get('item_edit')"));
    assert!(!html.contains("data-result-kind"));
    for legacy in ["page=commands", "page=pulse", "page=goals", "page=request"] {
        assert!(!html.contains(legacy));
    }
}

#[tokio::test]
async fn new_work_page_exposes_working_item_and_branch_creation_paths() {
    let (status, html) = text_response(
        test_server().router(),
        Request::builder()
            .uri("/?page=work&section=brief&entity=new&focus=create&lang=ja")
            .body(Body::empty())
            .expect("request"),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert!(html.contains("新しい Work の作成方法"));
    assert!(html.contains("data-create-work-from-branch=\"origin/main...HEAD\""));
    assert!(html.contains("Item から作成"));
    assert!(html.contains("data-work-lang=\"ja\""));
}

#[tokio::test]
async fn branch_creation_action_adds_a_selectable_work() {
    let server = test_server();
    let (status, action) = json_response(
        server.router(),
        Request::builder()
            .method("POST")
            .uri("/api/actions/branch.infer_goal/run")
            .header("content-type", "application/json")
            .body(Body::from(r#"{"range":"HEAD...HEAD"}"#))
            .expect("request"),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(action["result"]["goal"]["id"], "GOAL-BRANCH-001");

    let (page_status, html) = text_response(
        server.router(),
        Request::builder()
            .uri("/?page=work&section=brief&entity=GOAL-BRANCH-001&lang=en")
            .body(Body::empty())
            .expect("request"),
    )
    .await;
    assert_eq!(page_status, StatusCode::OK);
    assert!(html.contains("GOAL-BRANCH-001 Infer goal from branch"));
    assert!(html.contains("data-goal-rail=\"true\""));
    assert!(!html.contains("data-work-selector=\"true\""));

    let (second_status, second_action) = json_response(
        server.router(),
        Request::builder()
            .method("POST")
            .uri("/api/actions/branch.infer_goal/run")
            .header("content-type", "application/json")
            .body(Body::from(r#"{"range":"HEAD...HEAD"}"#))
            .expect("request"),
    )
    .await;
    assert_eq!(second_status, StatusCode::OK);
    assert_eq!(second_action["result"]["goal"]["id"], "GOAL-BRANCH-002");
}

#[tokio::test]
async fn item_search_filters_results_and_draft_id_is_submitted() {
    let server = test_server();
    let (_, filtered) = text_response(
        server.router(),
        Request::builder()
            .uri("/?page=items&section=requirement&spec_query=definitely-not-an-item")
            .body(Body::empty())
            .expect("request"),
    )
    .await;
    assert!(filtered.contains("No Items in this layer"));
    assert!(!filtered.contains("REQ-WORKBENCH-001 ·"));

    let (_, draft) = text_response(
        server.router(),
        Request::builder()
            .uri("/?page=items&section=requirement&entity=draft")
            .body(Body::empty())
            .expect("request"),
    )
    .await;
    assert!(draft.contains("name=\"item_edit\""));
    assert!(draft.contains("value=\"REQ-NEW-001\""));
    assert!(draft.contains(" required"));

    let (preview_status, preview) = text_response(
        server.router(),
        Request::builder()
            .method("POST")
            .uri("/run")
            .header("host", "127.0.0.1:3000")
            .header("origin", "http://127.0.0.1:3000")
            .header("content-type", "application/x-www-form-urlencoded")
            .body(Body::from("page=items&section=requirement&lang=ja&entity=REQ-NEW-001&item_edit=REQ-CODEX-PREVIEW-001&title=Preview+item&status=planned&item_edit_apply=0"))
            .expect("request"),
    )
    .await;
    assert_eq!(preview_status, StatusCode::OK);
    assert!(preview.contains("REQ-CODEX-PREVIEW-001"));
    assert!(preview.contains("Preview item"));
    assert!(preview.contains(">適用</button>"));
}

#[tokio::test]
async fn page_section_entity_and_locale_restore_from_query() {
    let (status, html) = text_response(
        test_server().router(),
        Request::builder()
            .uri("/?page=items&section=feature&entity=draft&focus=create&lang=ja")
            .body(Body::empty())
            .expect("request"),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert!(html.contains("lang=\"ja\""));
    assert!(html.contains("既存 Item と同じ Detail Canvas"));
    assert!(html.contains("border-red-500"));
    assert!(html.contains("data-command-target=\"item-editor\""));
    assert!(!html.contains("name=\"cli\""));
}

#[tokio::test]
async fn legacy_page_slug_does_not_restore_a_compatibility_surface() {
    let (status, html) = text_response(
        test_server().router(),
        Request::builder()
            .uri("/?page=commands")
            .body(Body::empty())
            .expect("request"),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert!(html.contains("Understand, assign, and verify implementation work"));
    assert!(!html.contains("generic result"));
}

#[tokio::test]
async fn diagnostics_run_exposes_job_lifecycle_and_evidence() {
    let server = test_server();
    let (status, json) = json_response(
        server.router(),
        Request::builder()
            .method("POST")
            .uri("/api/diagnostics/run")
            .body(Body::empty())
            .expect("request"),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json["status"], "queued");
    let job_id = json["id"].as_str().expect("job id").to_string();

    for _ in 0..40 {
        if server
            .inner
            .jobs
            .read()
            .await
            .get(&job_id)
            .is_some_and(|job| job.status == "completed")
        {
            break;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    let jobs = server.inner.jobs.read().await;
    assert_eq!(
        jobs.get(&job_id).map(|job| job.status.as_str()),
        Some("completed")
    );
    drop(jobs);
    let state = server.inner.state.read().await;
    assert_eq!(state.job.status, JobStatus::Completed);
    assert!(
        state
            .evidence_timeline
            .entries
            .iter()
            .any(|entry| entry.kind == WorkbenchEvidenceKind::ValidationReport)
    );
}

#[tokio::test]
async fn workbench_mutation_rejects_cross_origin_posts() {
    let response = test_server()
        .router()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/run")
                .header("host", "127.0.0.1:3000")
                .header("origin", "http://attacker.example")
                .header("content-type", "application/x-www-form-urlencoded")
                .body(Body::from("page=items"))
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn health_and_actions_endpoints_report_live_server_state() {
    let server = test_server();
    let (health_status, health) = json_response(
        server.router(),
        Request::builder()
            .uri("/api/health")
            .body(Body::empty())
            .expect("request"),
    )
    .await;
    assert_eq!(health_status, StatusCode::OK);
    assert_eq!(health["ok"], true);
    let (actions_status, actions) = json_response(
        server.router(),
        Request::builder()
            .uri("/api/actions")
            .body(Body::empty())
            .expect("request"),
    )
    .await;
    assert_eq!(actions_status, StatusCode::OK);
    assert!(
        actions["actions"]
            .as_array()
            .is_some_and(|items| !items.is_empty())
    );
}

#[tokio::test]
async fn request_plan_and_branch_scope_feed_page_models() {
    let server = test_server();
    let body = serde_json::json!({
        "request": {
            "version": 1,
            "request": "Add Workbench planning coverage",
            "context": { "linked_ids": ["REQ-WORKBENCH-006"] }
        },
        "request_path": "request.yaml",
        "kind": "govern",
        "operation": "modify"
    });
    let (plan_status, plan) = json_response(
        server.router(),
        Request::builder()
            .method("POST")
            .uri("/api/request/plan")
            .header("content-type", "application/json")
            .body(Body::from(body.to_string()))
            .expect("request"),
    )
    .await;
    assert_eq!(plan_status, StatusCode::OK);
    assert_eq!(plan["kind"], "syu.goal_plan");
    assert_eq!(plan["work"]["intent"]["kind"], "govern");
    assert_eq!(plan["work"]["intent"]["operation"], "modify");
    assert!(plan["work"]["verification"]["completion"].is_array());
    let artifact: GoalPlanArtifact = serde_json::from_value(plan).expect("typed goal plan");
    {
        let mut state = server.inner.state.write().await;
        state.goals.selected_goal_id = Some(artifact.goal.id.clone());
        state.goals.active.push(ActiveGoalState {
            goal_id: artifact.goal.id.clone(),
            goal_plan: Some(artifact),
            ..ActiveGoalState::default()
        });
    }
    let (_, work_html) = text_response(
        server.router(),
        Request::builder()
            .uri("/?page=work&lang=en")
            .body(Body::empty())
            .expect("request"),
    )
    .await;
    assert!(work_html.contains("data-work-kind=\"govern\""));
    assert!(work_html.contains("data-work-section=\"impact\""));
    assert!(work_html.contains("data-work-section=\"verification\""));
    assert!(!work_html.contains("data-work-selector=\"true\""));

    let (scope_status, scope) = json_response(
        server.router(),
        Request::builder()
            .uri("/api/branch/scope?range=HEAD...HEAD")
            .body(Body::empty())
            .expect("request"),
    )
    .await;
    assert_eq!(scope_status, StatusCode::OK);
    assert_eq!(scope["range"], "HEAD...HEAD");
}

#[tokio::test]
async fn validation_action_records_structured_evidence() {
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
    let snapshot = server.inner.state.read().await;
    assert!(snapshot.evidence_timeline.entries.iter().any(|entry| {
        entry.kind == WorkbenchEvidenceKind::ValidationReport
            && entry.status == EvidenceStatus::Pass
    }));
}

#[tokio::test]
async fn item_driven_work_records_item_source_and_goal() {
    let server = test_server();
    let (status, plan) = json_response(
        server.router(),
        Request::builder()
            .method("POST")
            .uri("/api/items/REQ-WORKBENCH-001/work")
            .body(Body::empty())
            .expect("request"),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(plan["source"]["mode"], "item_driven");
    assert_eq!(plan["source"]["evidence"]["item_id"], "REQ-WORKBENCH-001");
    let state = server.inner.state.read().await;
    assert!(
        state
            .goals
            .active
            .iter()
            .any(|goal| goal.goal_id == "GOAL-REQ-WORKBENCH-001")
    );
    drop(state);
    let (repeat_status, repeat_plan) = json_response(
        server.router(),
        Request::builder()
            .method("POST")
            .uri("/api/items/REQ-WORKBENCH-001/work")
            .body(Body::empty())
            .expect("request"),
    )
    .await;
    assert_eq!(repeat_status, StatusCode::OK);
    assert_eq!(repeat_plan["goal"]["id"], "GOAL-REQ-WORKBENCH-001");
    assert_eq!(
        server
            .inner
            .state
            .read()
            .await
            .goals
            .active
            .iter()
            .filter(|goal| goal.goal_id == "GOAL-REQ-WORKBENCH-001")
            .count(),
        1
    );
}

#[test]
fn item_draft_append_uses_existing_yaml_collection_without_rewriting_it() {
    let raw = "# header\ncategory: Core\nrequirements:\n  - id: REQ-OLD-001\n    title: Existing\n";
    let item = serde_yaml::from_str::<serde_yaml::Value>(
        "id: REQ-NEW-001\ntitle: New requirement\nstatus: planned\n",
    )
    .expect("item");
    let updated = append_yaml_item_block(raw, SectionKind::Requirements, &item).expect("append");
    assert!(updated.starts_with("# header\n"));
    assert!(updated.contains("  - id: REQ-OLD-001"));
    assert!(updated.contains("  - id: REQ-NEW-001"));
}

#[tokio::test]
async fn settings_preview_and_apply_preserve_comments_and_unknown_fields() {
    let tempdir = tempfile::tempdir().expect("tempdir");
    fs::write(
        tempdir.path().join("syu.yaml"),
        "# keep this comment\nversion: 0.0.1-alpha.8\nspec:\n  root: docs/syu\ncustom_field: keep\n",
    ).expect("write config");
    let server = WorkbenchServer::new(WorkbenchLaunchConfig {
        workspace_root: tempdir.path().to_path_buf(),
        spec_root: tempdir.path().join("docs/syu"),
        bind: "127.0.0.1".to_string(),
        port: 3000,
        allow_remote_bind: false,
        show_log: false,
    })
    .expect("server");
    let update = serde_json::json!({
        "spec_root": "docs/spec",
        "bind": "127.0.0.1",
        "port": 3100,
        "strict_review": true
    });
    let (preview_status, preview) = json_response(
        server.router(),
        Request::builder()
            .method("POST")
            .uri("/api/settings/preview")
            .header("content-type", "application/json")
            .body(Body::from(update.to_string()))
            .expect("request"),
    )
    .await;
    assert_eq!(preview_status, StatusCode::OK);
    assert!(
        preview["diff"]
            .as_str()
            .is_some_and(|diff| diff.contains("strict_review: true"))
    );
    let mut apply = update;
    apply["source_hash"] = preview["source_hash"].clone();
    let (apply_status, applied) = json_response(
        server.router(),
        Request::builder()
            .method("POST")
            .uri("/api/settings/apply")
            .header("host", "127.0.0.1:3000")
            .header("origin", "http://127.0.0.1:3000")
            .header("content-type", "application/json")
            .body(Body::from(apply.to_string()))
            .expect("request"),
    )
    .await;
    assert_eq!(apply_status, StatusCode::OK);
    assert_eq!(applied["applied"], true);
    let rendered = fs::read_to_string(tempdir.path().join("syu.yaml")).expect("read config");
    assert!(rendered.contains("# keep this comment"));
    assert!(rendered.contains("custom_field: keep"));
    assert!(rendered.contains("root: docs/spec"));
}

#[tokio::test]
async fn css_and_event_stream_are_available() {
    let server = test_server();
    let (css_status, css) = text_response(
        server.router(),
        Request::builder()
            .uri("/assets/tailwind.css")
            .body(Body::empty())
            .expect("request"),
    )
    .await;
    assert_eq!(css_status, StatusCode::OK);
    assert!(css.contains("@layer theme"));

    let response = server
        .router()
        .oneshot(
            Request::builder()
                .uri("/api/events")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response
            .headers()
            .get(axum::http::header::CONTENT_TYPE)
            .and_then(|value| value.to_str().ok()),
        Some("text/event-stream")
    );
}
