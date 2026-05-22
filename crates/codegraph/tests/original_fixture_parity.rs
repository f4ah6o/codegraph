mod support;

use codegraph::extraction::registered_extractor_name;
use codegraph::types::{EdgeKind, Language, NodeKind, SearchOptions};
use support::{OriginalFixtureProject, OriginalSourceFixture};

#[test]
fn harness_can_assert_extractor_registry_dispatch() {
    assert_eq!(registered_extractor_name(Language::Rust), "rust");
    assert_eq!(registered_extractor_name(Language::MoonBit), "moonbit");
    assert_eq!(
        registered_extractor_name(Language::TypeScript),
        "typescript_javascript"
    );
    assert_eq!(
        registered_extractor_name(Language::JavaScript),
        "typescript_javascript"
    );
    assert_eq!(registered_extractor_name(Language::Python), "python");
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
    fixture.assert_exported_node(NodeKind::Function, "processPayment");
    fixture.assert_exported_node(NodeKind::Class, "PaymentService");
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
fn harness_extracts_typescript_interfaces_aliases_and_imports() {
    let fixture = OriginalSourceFixture::new(
        "types.ts",
        r#"
import React, { useState as useStateAlias } from 'react';
import type { FC, ReactNode } from 'react';
import './styles.css';

export interface User {
  id: string;
}

export type AuthContextValue = {
  user: User | null;
};

export const useAuth = (): AuthContextValue => {
  return useContext(AuthContext);
};
"#,
    );

    fixture.assert_exported_node(NodeKind::Interface, "User");
    fixture.assert_exported_node(NodeKind::TypeAlias, "AuthContextValue");
    fixture.assert_exported_node(NodeKind::Function, "useAuth");
    fixture.assert_node(NodeKind::Import, "react");
    fixture.assert_node(NodeKind::Import, "./styles.css");
    fixture.assert_reference(EdgeKind::Imports, "react");
    fixture.assert_reference(EdgeKind::Imports, "./styles.css");
}

#[test]
fn harness_extracts_jsx_components_and_javascript_arrows() {
    let jsx = OriginalSourceFixture::new(
        "Button.jsx",
        r#"
import * as React from 'react';

export const Button = async () => {
  return <button onClick={trackClick}>Save</button>;
};
"#,
    );

    jsx.assert_exported_node(NodeKind::Function, "Button");
    jsx.assert_exported_node(NodeKind::Component, "Button");
    jsx.assert_reference(EdgeKind::Imports, "react");

    let js = OriginalSourceFixture::new(
        "api.js",
        r#"
export const fetchData = async () => {
  const response = await fetch('/api/data');
  return response.json();
};
"#,
    );

    js.assert_exported_node(NodeKind::Function, "fetchData");
    js.assert_reference(EdgeKind::Calls, "fetch");
}

#[test]
fn harness_extracts_python_symbols_decorators_and_imports() {
    let fixture = OriginalSourceFixture::new(
        "views.py",
        r#"
from .utils import helper
from typing import List, Dict, Optional
from typing import *
import json
import numpy as np

class Cart:
    @staticmethod
    def tax_rate():
        return 0.1

    @app.get("/items/{item_id}")
    async def fetch_item(self, item_id: str) -> dict:
        return helper(item_id)

def calculate_total(items: list, tax_rate: float) -> float:
    return sum(items) * tax_rate
"#,
    );

    assert_eq!(fixture.language(), Language::Python);
    fixture.assert_node(NodeKind::Class, "Cart");
    fixture.assert_node(NodeKind::Method, "tax_rate");
    fixture.assert_node(NodeKind::Method, "fetch_item");
    fixture.assert_node(NodeKind::Function, "calculate_total");
    fixture.assert_node(NodeKind::Import, ".utils");
    fixture.assert_node(NodeKind::Import, "typing");
    fixture.assert_node(NodeKind::Import, "json");
    fixture.assert_node(NodeKind::Import, "numpy");
    fixture.assert_reference(EdgeKind::Imports, ".utils");
    fixture.assert_reference(EdgeKind::Imports, "typing");
    fixture.assert_reference(EdgeKind::Imports, "json");
    fixture.assert_reference(EdgeKind::Imports, "numpy");
    fixture.assert_reference(EdgeKind::Decorates, "staticmethod");
    fixture.assert_reference(EdgeKind::Decorates, "app.get");

    let tax_rate = fixture
        .result()
        .nodes
        .iter()
        .find(|node| node.kind == NodeKind::Method && node.name == "tax_rate")
        .unwrap();
    assert!(tax_rate.is_static);

    let fetch_item = fixture
        .result()
        .nodes
        .iter()
        .find(|node| node.kind == NodeKind::Method && node.name == "fetch_item")
        .unwrap();
    assert!(fetch_item.is_async);
    assert_eq!(fetch_item.qualified_name, "Cart.fetch_item");
    assert!(fetch_item
        .signature
        .as_deref()
        .is_some_and(|signature| signature.contains("@app.get")));
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
