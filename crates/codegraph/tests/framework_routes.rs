use codegraph::types::{EdgeKind, NodeKind, SearchOptions};
use codegraph::CodeGraph;
use std::fs;
use tempfile::TempDir;

#[test]
fn moonbit_sol_routes_link_to_handlers() {
    let dir = TempDir::new().unwrap();
    fs::write(
        dir.path().join("moon.mod.json"),
        r#"{"name":"example/luna"}"#,
    )
    .unwrap();
    fs::write(dir.path().join("moon.pkg.json"), "{}").unwrap();
    fs::write(
        dir.path().join("main.mbt"),
        r#"
pub fn home_page() -> String {
  "ok"
}

pub fn api_health() -> String {
  "ok"
}

let routes = [
  @sol.route("/", home_page),
  @sol.api_get("/api/health", api_health),
  // @sol.api_post("/fake", fake_handler)
]
"#,
    )
    .unwrap();

    let mut cg = CodeGraph::init(dir.path()).unwrap();
    let index = cg.index_all().unwrap();
    assert!(index.success, "{:?}", index.errors);

    let routes = cg
        .search_nodes(
            "GET /api/health",
            SearchOptions {
                kind: Some(NodeKind::Route),
                limit: 5,
                ..Default::default()
            },
        )
        .unwrap();
    assert_eq!(routes.len(), 1, "{routes:?}");

    let callees = cg.get_callees(&routes[0].node.id, 1).unwrap();
    assert!(
        callees.iter().any(|entry| {
            entry.node.name == "api_health" && entry.edge.kind == EdgeKind::References
        }),
        "{callees:?}"
    );

    let fake = cg
        .search_nodes(
            "/fake",
            SearchOptions {
                kind: Some(NodeKind::Route),
                limit: 5,
                ..Default::default()
            },
        )
        .unwrap();
    assert!(fake.is_empty(), "{fake:?}");
}

#[test]
fn js_routes_and_filesystem_routes_are_indexed() {
    let dir = TempDir::new().unwrap();
    fs::write(
        dir.path().join("package.json"),
        r#"{"dependencies":{"express":"latest","next":"latest","@sveltejs/kit":"latest"}}"#,
    )
    .unwrap();
    fs::create_dir_all(dir.path().join("src/server")).unwrap();
    fs::write(
        dir.path().join("src/server/routes.ts"),
        r#"
export function listUsers() {
  return []
}

router.get("/users", auth, listUsers)
// router.post("/fake", fakeHandler)
const docs = "http://example.test/not-a-comment"
"#,
    )
    .unwrap();
    fs::create_dir_all(dir.path().join("src/routes/blog/[slug]")).unwrap();
    fs::write(
        dir.path().join("src/routes/blog/[slug]/+page.svelte"),
        "<script>export let data;</script>",
    )
    .unwrap();
    fs::create_dir_all(dir.path().join("app/dashboard")).unwrap();
    fs::write(
        dir.path().join("app/dashboard/page.tsx"),
        "export default function Dashboard() { return <main /> }\n",
    )
    .unwrap();
    fs::create_dir_all(dir.path().join("pages")).unwrap();
    fs::write(
        dir.path().join("pages/_app.tsx"),
        "export default function App() { return null }\n",
    )
    .unwrap();
    fs::create_dir_all(dir.path().join("docs/pages")).unwrap();
    fs::write(
        dir.path().join("docs/pages/intro.tsx"),
        "export function Intro() {}\n",
    )
    .unwrap();

    let mut cg = CodeGraph::init(dir.path()).unwrap();
    let index = cg.index_all().unwrap();
    assert!(index.success, "{:?}", index.errors);

    let users = cg
        .search_nodes(
            "GET /users",
            SearchOptions {
                kind: Some(NodeKind::Route),
                limit: 5,
                ..Default::default()
            },
        )
        .unwrap();
    assert_eq!(users.len(), 1, "{users:?}");
    let callees = cg.get_callees(&users[0].node.id, 1).unwrap();
    assert!(
        callees.iter().any(|entry| entry.node.name == "listUsers"),
        "{callees:?}"
    );

    let sveltekit = cg
        .search_nodes(
            "/blog/:slug",
            SearchOptions {
                kind: Some(NodeKind::Route),
                limit: 5,
                ..Default::default()
            },
        )
        .unwrap();
    assert_eq!(sveltekit.len(), 1, "{sveltekit:?}");

    let next = cg
        .search_nodes(
            "/dashboard",
            SearchOptions {
                kind: Some(NodeKind::Route),
                limit: 5,
                ..Default::default()
            },
        )
        .unwrap();
    assert_eq!(next.len(), 1, "{next:?}");

    let fake = cg
        .search_nodes(
            "/fake",
            SearchOptions {
                kind: Some(NodeKind::Route),
                limit: 5,
                ..Default::default()
            },
        )
        .unwrap();
    assert!(fake.is_empty(), "{fake:?}");

    let next_internal = cg
        .search_nodes(
            "/_app",
            SearchOptions {
                kind: Some(NodeKind::Route),
                limit: 5,
                ..Default::default()
            },
        )
        .unwrap();
    assert!(next_internal.is_empty(), "{next_internal:?}");

    let docs_page = cg
        .search_nodes(
            "/intro",
            SearchOptions {
                kind: Some(NodeKind::Route),
                limit: 5,
                ..Default::default()
            },
        )
        .unwrap();
    assert!(docs_page.is_empty(), "{docs_page:?}");
}
