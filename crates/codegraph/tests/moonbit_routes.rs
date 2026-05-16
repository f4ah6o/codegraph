use serde_json::Value;
use std::fs;
use std::process::Command;
use tempfile::TempDir;

fn run(args: &[&str]) -> String {
    let bin = env!("CARGO_BIN_EXE_cgz");
    let output = Command::new(bin)
        .args(args)
        .output()
        .expect("failed to run cgz");
    assert!(
        output.status.success(),
        "cgz failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).unwrap()
}

fn query(project: &str, term: &str) -> Value {
    serde_json::from_str(&run(&["query", term, "--path", project, "--json"])).unwrap()
}

fn route_names(value: &Value) -> Vec<String> {
    value
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|r| {
            let node = &r["node"];
            (node["kind"].as_str() == Some("route"))
                .then(|| node["name"].as_str().unwrap().to_string())
        })
        .collect()
}

#[test]
fn extracts_sol_routes_and_resolves_handlers() {
    let dir = TempDir::new().unwrap();
    fs::write(
        dir.path().join("moon.mod.json"),
        r#"{"name":"example/sol-routes"}"#,
    )
    .unwrap();
    fs::write(dir.path().join("moon.pkg.json"), "{}").unwrap();
    fs::write(
        dir.path().join("routes.mbt"),
        r#"
pub fn routes() -> Array[@sol.SolRoutes] {
  [
    @sol.wrap("/admin", admin_layout, [
      @sol.route("/", admin_home, title="Admin"),
      @sol.route("/users", admin_users, title="Users"),
    ]),
    @sol.route("/", home, title="Home"),
    @sol.api_get("/api/items/:id", get_item_handler),
    @sol.api_post("/api/items", create_item_handler),
    @router.raw_get("/media/:id", serve_media),
    @router.page(path="/about", handler=about_page, title="About"),
    SolRoutes::Page(path="/contact", handler=contact_page, title="Contact"),
    SolRoutes::RawPost(path="/upload", handler=upload_media),
    // @sol.api_get("/api/commented", commented_handler)
    /* @sol.route("/block-commented", block_commented) */
  ]
}

async fn admin_layout(_props : @sol.PageProps, content : @server_dom.ServerNode) -> @server_dom.ServerNode { content }
async fn admin_home(_props : @sol.PageProps) -> @server_dom.ServerNode { todo() }
async fn admin_users(_props : @sol.PageProps) -> @server_dom.ServerNode { todo() }
async fn home(_props : @sol.PageProps) -> @server_dom.ServerNode { todo() }
async fn get_item_handler(_props : @sol.PageProps) -> Json { "{}" }
async fn create_item_handler(_props : @sol.PageProps) -> Json { "{}" }
async fn serve_media(_props : @sol.PageProps) -> @http.Response { response() }
async fn about_page(_props : @sol.PageProps) -> @server_dom.ServerNode { todo() }
async fn contact_page(_props : @sol.PageProps) -> @server_dom.ServerNode { todo() }
async fn upload_media(_props : @sol.PageProps) -> @http.Response { response() }
"#,
    )
    .unwrap();

    let project = dir.path().to_str().unwrap();
    run(&["init", project, "--index"]);

    let api = query(project, "/api/items");
    let api_routes = route_names(&api);
    assert!(api_routes.iter().any(|name| name == "GET /api/items/:id"));
    assert!(api_routes.iter().any(|name| name == "POST /api/items"));

    let admin = query(project, "/admin");
    let admin_routes = route_names(&admin);
    assert!(admin_routes.iter().any(|name| name == "PAGE /admin"));
    assert!(admin_routes.iter().any(|name| name == "PAGE /admin/users"));

    let raw = query(project, "/media");
    assert!(route_names(&raw)
        .iter()
        .any(|name| name == "RAW GET /media/:id"));

    let constructor = query(project, "/upload");
    assert!(route_names(&constructor)
        .iter()
        .any(|name| name == "RAW POST /upload"));

    let commented = query(project, "commented");
    assert!(
        route_names(&commented).is_empty(),
        "commented routes should not be indexed: {commented}"
    );

    let handler = query(project, "get_item_handler");
    assert!(handler.as_array().unwrap().iter().any(|r| {
        let node = &r["node"];
        node["kind"].as_str() == Some("function")
            && node["name"].as_str() == Some("get_item_handler")
    }));
}
