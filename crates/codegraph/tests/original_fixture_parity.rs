mod support;

use codegraph::extraction::registered_extractor_name;
use codegraph::types::{EdgeKind, Language, NodeKind, SearchOptions};
use support::{OriginalFixtureProject, OriginalSourceFixture};

#[test]
fn harness_can_assert_extractor_registry_dispatch() {
    assert_eq!(registered_extractor_name(Language::Rust), "rust");
    assert_eq!(registered_extractor_name(Language::MoonBit), "moonbit");
    assert_eq!(registered_extractor_name(Language::TypeScript), "generic");
}

#[test]
fn harness_extracts_original_style_typescript_symbols() {
    let fixture = OriginalSourceFixture::new(
        "payment.ts",
        r#"
import { StripeClient } from "./stripe";

export function processPayment(amount: number): Promise<Receipt> {
  return stripe.charge(amount);
}

export class PaymentService {
  async charge(amount: number): Promise<Receipt> {
    return this.stripe.charge(amount);
  }
}
"#,
    );

    assert_eq!(fixture.language(), Language::TypeScript);
    assert_eq!(fixture.path().to_string_lossy(), "payment.ts");
    assert!(fixture.source().contains("processPayment"));
    fixture.assert_node(NodeKind::Function, "processPayment");
    fixture.assert_node(NodeKind::Class, "PaymentService");
    fixture.assert_reference(EdgeKind::Calls, "stripe.charge");
    assert!(
        fixture
            .result()
            .edges
            .iter()
            .any(|edge| edge.kind == EdgeKind::Contains),
        "fixture should expose containment edges for shared assertions"
    );
}

#[test]
fn harness_extracts_import_fixture_references() {
    let fixture = OriginalSourceFixture::new(
        "moon.pkg.json",
        r#"{"import":{"runtime":"example/app/runtime"}}"#,
    );

    fixture.assert_node(NodeKind::Import, "runtime");
    fixture.assert_reference(EdgeKind::Imports, "runtime");
}

#[test]
fn harness_indexes_project_and_queries_fixture_nodes() {
    let project = OriginalFixtureProject::new(&[(
        "src/cache.rs",
        r#"
pub struct CacheStore {
    entries: Vec<String>,
}

pub fn evict_expired(store: &mut CacheStore) {
    store.entries.clear();
}
"#,
    )]);

    let cg = project.index();
    assert!(project.root().join(".codegraph").exists());

    let results = cg
        .search_nodes(
            "evict_expired",
            SearchOptions {
                limit: 5,
                ..Default::default()
            },
        )
        .unwrap();

    assert!(
        results.iter().any(|result| {
            result.node.kind == NodeKind::Function && result.node.name == "evict_expired"
        }),
        "{results:?}"
    );
}

#[test]
fn harness_supports_route_fixture_parity() {
    let project = OriginalFixtureProject::new(&[
        ("moon.mod.json", r#"{"name":"example/routes"}"#),
        ("moon.pkg.json", "{}"),
        (
            "routes.mbt",
            r#"
pub fn routes() -> Array[@sol.SolRoutes] {
  [
    @sol.route("/", home, title="Home"),
    @sol.api_get("/api/items/:id", get_item_handler),
  ]
}

async fn home(_props : @sol.PageProps) -> @server_dom.ServerNode { todo() }
async fn get_item_handler(_props : @sol.PageProps) -> Json { "{}" }
"#,
        ),
    ]);

    let cg = project.index();
    let results = cg
        .search_nodes(
            "/api/items",
            SearchOptions {
                limit: 10,
                ..Default::default()
            },
        )
        .unwrap();

    assert!(
        results
            .iter()
            .any(|result| result.node.kind == NodeKind::Route
                && result.node.name == "GET /api/items/:id"),
        "{results:?}"
    );
}
