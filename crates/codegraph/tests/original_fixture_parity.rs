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
    assert_eq!(registered_extractor_name(Language::Go), "go");
    assert_eq!(registered_extractor_name(Language::Java), "java_kotlin");
    assert_eq!(registered_extractor_name(Language::Kotlin), "java_kotlin");
    assert_eq!(registered_extractor_name(Language::CSharp), "csharp");
    assert_eq!(registered_extractor_name(Language::Php), "php_ruby");
    assert_eq!(registered_extractor_name(Language::Ruby), "php_ruby");
    assert_eq!(registered_extractor_name(Language::Swift), "swift");
    assert_eq!(
        registered_extractor_name(Language::Dart),
        "dart_pascal_scala"
    );
    assert_eq!(
        registered_extractor_name(Language::Pascal),
        "dart_pascal_scala"
    );
    assert_eq!(
        registered_extractor_name(Language::Scala),
        "dart_pascal_scala"
    );
    assert_eq!(
        registered_extractor_name(Language::Liquid),
        "liquid_vue_svelte"
    );
    assert_eq!(
        registered_extractor_name(Language::Vue),
        "liquid_vue_svelte"
    );
    assert_eq!(
        registered_extractor_name(Language::Svelte),
        "liquid_vue_svelte"
    );
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
fn harness_extracts_go_functions_methods_imports_and_calls() {
    let fixture = OriginalSourceFixture::new(
        "server.go",
        r#"
package server

import "net/http"
import alias "example.com/project/pkg"
import (
    "fmt"
    . "strings"
    _ "embed"
)

type Server struct {
    mux *http.ServeMux
}

type Handler interface {
    ServeHTTP(http.ResponseWriter, *http.Request)
}

func NewServer() *Server {
    fmt.Println(alias.Name)
    return &Server{}
}

func (s *Server) ServeHTTP(w http.ResponseWriter, r *http.Request) {
    TrimSpace(r.URL.Path)
    s.mux.ServeHTTP(w, r)
}
"#,
    );

    assert_eq!(fixture.language(), Language::Go);
    fixture.assert_node(NodeKind::Module, "server");
    fixture.assert_node(NodeKind::Struct, "Server");
    fixture.assert_node(NodeKind::Interface, "Handler");
    fixture.assert_node(NodeKind::Function, "NewServer");
    fixture.assert_node(NodeKind::Method, "ServeHTTP");
    fixture.assert_node(NodeKind::Import, "net/http");
    fixture.assert_node(NodeKind::Import, "example.com/project/pkg");
    fixture.assert_node(NodeKind::Import, "fmt");
    fixture.assert_node(NodeKind::Import, "strings");
    fixture.assert_node(NodeKind::Import, "embed");
    fixture.assert_reference(EdgeKind::Imports, "net/http");
    fixture.assert_reference(EdgeKind::Imports, "example.com/project/pkg");
    fixture.assert_reference(EdgeKind::Imports, "fmt");
    fixture.assert_reference(EdgeKind::Imports, "strings");
    fixture.assert_reference(EdgeKind::Imports, "embed");
    fixture.assert_reference(EdgeKind::Calls, "fmt.Println");
    fixture.assert_reference(EdgeKind::Calls, "TrimSpace");
    fixture.assert_reference(EdgeKind::Calls, "s.mux.ServeHTTP");

    let method = fixture
        .result()
        .nodes
        .iter()
        .find(|node| node.kind == NodeKind::Method && node.name == "ServeHTTP")
        .unwrap();
    assert_eq!(method.qualified_name, "Server.ServeHTTP");
    assert!(method
        .signature
        .as_deref()
        .is_some_and(|signature| signature.contains("func (s *Server) ServeHTTP")));

    let aliased_import = fixture
        .result()
        .nodes
        .iter()
        .find(|node| node.kind == NodeKind::Import && node.name == "example.com/project/pkg")
        .unwrap();
    assert!(aliased_import
        .signature
        .as_deref()
        .is_some_and(|signature| signature.starts_with("import alias")));
}

#[test]
fn harness_extracts_java_symbols_annotations_inheritance_and_imports() {
    let fixture = OriginalSourceFixture::new(
        "PaymentController.java",
        r#"
package app;

import java.util.List;
import static java.util.Collections.emptyList;

@RestController
public class PaymentController extends BaseController implements Handler, Auditable {
    @GetMapping("/payments")
    public static List<String> listPayments() {
        return emptyList();
    }
}
"#,
    );

    assert_eq!(fixture.language(), Language::Java);
    fixture.assert_exported_node(NodeKind::Class, "PaymentController");
    fixture.assert_node(NodeKind::Method, "listPayments");
    fixture.assert_node(NodeKind::Import, "java.util.List");
    fixture.assert_node(NodeKind::Import, "java.util.Collections.emptyList");
    fixture.assert_reference(EdgeKind::Imports, "java.util.List");
    fixture.assert_reference(EdgeKind::Imports, "java.util.Collections.emptyList");
    fixture.assert_reference(EdgeKind::Extends, "BaseController");
    fixture.assert_reference(EdgeKind::Implements, "Handler");
    fixture.assert_reference(EdgeKind::Implements, "Auditable");
    fixture.assert_reference(EdgeKind::Decorates, "RestController");
    fixture.assert_reference(EdgeKind::Decorates, "GetMapping");
    fixture.assert_reference(EdgeKind::Calls, "emptyList");

    let method = fixture
        .result()
        .nodes
        .iter()
        .find(|node| node.kind == NodeKind::Method && node.name == "listPayments")
        .unwrap();
    assert!(method.is_static);
    assert_eq!(method.visibility.as_deref(), Some("public"));
    assert_eq!(method.qualified_name, "PaymentController.listPayments");
}

#[test]
fn harness_extracts_kotlin_symbols_suspend_annotations_and_imports() {
    let fixture = OriginalSourceFixture::new(
        "PaymentService.kt",
        r#"
package app

import kotlinx.coroutines.Dispatchers
import app.routes.*

@Service
class PaymentService : BaseService {
    @GetMapping("/payments")
    suspend fun listPayments(): List<String> {
        return fetchPayments()
    }
}

fun helperName(value: String): String {
    return value.trim()
}
"#,
    );

    assert_eq!(fixture.language(), Language::Kotlin);
    fixture.assert_exported_node(NodeKind::Class, "PaymentService");
    fixture.assert_node(NodeKind::Method, "listPayments");
    fixture.assert_node(NodeKind::Function, "helperName");
    fixture.assert_node(NodeKind::Import, "kotlinx.coroutines.Dispatchers");
    fixture.assert_node(NodeKind::Import, "app.routes.*");
    fixture.assert_reference(EdgeKind::Imports, "kotlinx.coroutines.Dispatchers");
    fixture.assert_reference(EdgeKind::Imports, "app.routes.*");
    fixture.assert_reference(EdgeKind::Extends, "BaseService");
    fixture.assert_reference(EdgeKind::Decorates, "Service");
    fixture.assert_reference(EdgeKind::Decorates, "GetMapping");
    fixture.assert_reference(EdgeKind::Calls, "fetchPayments");

    let method = fixture
        .result()
        .nodes
        .iter()
        .find(|node| node.kind == NodeKind::Method && node.name == "listPayments")
        .unwrap();
    assert!(method.is_async);
    assert_eq!(method.qualified_name, "PaymentService.listPayments");
}

#[test]
fn harness_extracts_csharp_symbols_attributes_inheritance_and_usings() {
    let fixture = OriginalSourceFixture::new(
        "PaymentsController.cs",
        r#"
using System.Collections.Generic;
using static System.Math;
using Json = System.Text.Json.JsonSerializer;

[ApiController]
public class PaymentsController : ControllerBase, IPaymentsController {
    public string Name { get; set; }

    [HttpGet("/payments")]
    public static async Task<List<string>> ListPayments() {
        return Json.Deserialize<List<string>>("[]");
    }
}

public interface IPaymentsController {
    Task<List<string>> ListPayments();
}
"#,
    );

    assert_eq!(fixture.language(), Language::CSharp);
    fixture.assert_exported_node(NodeKind::Class, "PaymentsController");
    fixture.assert_exported_node(NodeKind::Interface, "IPaymentsController");
    fixture.assert_node(NodeKind::Property, "Name");
    fixture.assert_node(NodeKind::Method, "ListPayments");
    fixture.assert_node(NodeKind::Import, "System.Collections.Generic");
    fixture.assert_node(NodeKind::Import, "System.Math");
    fixture.assert_node(NodeKind::Import, "System.Text.Json.JsonSerializer");
    fixture.assert_reference(EdgeKind::Imports, "System.Collections.Generic");
    fixture.assert_reference(EdgeKind::Imports, "System.Math");
    fixture.assert_reference(EdgeKind::Imports, "System.Text.Json.JsonSerializer");
    fixture.assert_reference(EdgeKind::Extends, "ControllerBase");
    fixture.assert_reference(EdgeKind::Implements, "IPaymentsController");
    fixture.assert_reference(EdgeKind::Decorates, "ApiController");
    fixture.assert_reference(EdgeKind::Decorates, "HttpGet");
    fixture.assert_reference(EdgeKind::Calls, "Json.Deserialize");

    let method = fixture
        .result()
        .nodes
        .iter()
        .find(|node| node.kind == NodeKind::Method && node.name == "ListPayments")
        .unwrap();
    assert!(method.is_static);
    assert!(method.is_async);
    assert_eq!(method.visibility.as_deref(), Some("public"));
    assert_eq!(method.qualified_name, "PaymentsController.ListPayments");
}

#[test]
fn harness_extracts_php_symbols_uses_inheritance_and_calls() {
    let fixture = OriginalSourceFixture::new(
        "PaymentController.php",
        r#"
<?php

use App\Services\PaymentService;
use App\Support\{Logger, Auditor};

class PaymentController extends BaseController implements Responsable, Jsonable {
    use AuthorizesRequests;

    public static function index($request) {
        return PaymentService::list($request);
    }
}

function payment_helper() {
    return Logger::debug('payments');
}
"#,
    );

    assert_eq!(fixture.language(), Language::Php);
    fixture.assert_exported_node(NodeKind::Class, "PaymentController");
    fixture.assert_node(NodeKind::Method, "index");
    fixture.assert_node(NodeKind::Function, "payment_helper");
    fixture.assert_node(NodeKind::Import, r"App\Services\PaymentService");
    fixture.assert_node(NodeKind::Import, r"App\Support\Logger");
    fixture.assert_node(NodeKind::Import, r"App\Support\Auditor");
    fixture.assert_reference(EdgeKind::Imports, r"App\Services\PaymentService");
    fixture.assert_reference(EdgeKind::Imports, r"App\Support\Logger");
    fixture.assert_reference(EdgeKind::Imports, r"App\Support\Auditor");
    fixture.assert_reference(EdgeKind::Extends, "BaseController");
    fixture.assert_reference(EdgeKind::Implements, "Responsable");
    fixture.assert_reference(EdgeKind::Implements, "Jsonable");
    fixture.assert_reference(EdgeKind::Implements, "AuthorizesRequests");
    fixture.assert_reference(EdgeKind::Calls, r"PaymentService::list");
    fixture.assert_reference(EdgeKind::Calls, r"Logger::debug");

    let method = fixture
        .result()
        .nodes
        .iter()
        .find(|node| node.kind == NodeKind::Method && node.name == "index")
        .unwrap();
    assert!(method.is_static);
    assert_eq!(method.visibility.as_deref(), Some("public"));
    assert_eq!(method.qualified_name, "PaymentController::index");
}

#[test]
fn harness_extracts_ruby_modules_classes_methods_requires_and_calls() {
    let fixture = OriginalSourceFixture::new(
        "payments_controller.rb",
        r#"
require "json"
require_relative "../models/payment"

module Admin
  class PaymentsController < ApplicationController
    def self.index
      render_json(payments)
    end

    def show
      Payment.find(params[:id])
    end
  end
end
"#,
    );

    assert_eq!(fixture.language(), Language::Ruby);
    fixture.assert_node(NodeKind::Import, "json");
    fixture.assert_node(NodeKind::Import, "../models/payment");
    fixture.assert_node(NodeKind::Module, "Admin");
    fixture.assert_node(NodeKind::Class, "PaymentsController");
    fixture.assert_node(NodeKind::Method, "index");
    fixture.assert_node(NodeKind::Method, "show");
    fixture.assert_reference(EdgeKind::Imports, "json");
    fixture.assert_reference(EdgeKind::Imports, "../models/payment");
    fixture.assert_reference(EdgeKind::Extends, "ApplicationController");
    fixture.assert_reference(EdgeKind::Calls, "render_json");
    fixture.assert_reference(EdgeKind::Calls, "find");

    let index = fixture
        .result()
        .nodes
        .iter()
        .find(|node| node.kind == NodeKind::Method && node.name == "index")
        .unwrap();
    assert!(index.is_static);
    assert_eq!(index.qualified_name, "PaymentsController.index");
}

#[test]
fn harness_extracts_swift_symbols_imports_conformance_and_calls() {
    let fixture = OriginalSourceFixture::new(
        "PaymentsView.swift",
        r#"
import SwiftUI
import Vapor

public protocol Routable {
    func routes()
}

public class PaymentsController: RouteCollection, Routable {
    public static func boot(routes: RoutesBuilder) async throws {
        routes.get("payments", use: index)
    }

    private func index(req: Request) -> String {
        return renderPayment()
    }
}

struct PaymentsView: View {
    var body: some View {
        Text("Payments")
    }
}

public typealias Handler = (Request) -> String

func renderPayment() -> String {
    return "ok"
}
"#,
    );

    assert_eq!(fixture.language(), Language::Swift);
    fixture.assert_node(NodeKind::Import, "SwiftUI");
    fixture.assert_node(NodeKind::Import, "Vapor");
    fixture.assert_exported_node(NodeKind::Protocol, "Routable");
    fixture.assert_exported_node(NodeKind::Class, "PaymentsController");
    fixture.assert_node(NodeKind::Struct, "PaymentsView");
    fixture.assert_exported_node(NodeKind::TypeAlias, "Handler");
    fixture.assert_node(NodeKind::Method, "boot");
    fixture.assert_node(NodeKind::Method, "index");
    fixture.assert_node(NodeKind::Function, "renderPayment");
    fixture.assert_reference(EdgeKind::Imports, "SwiftUI");
    fixture.assert_reference(EdgeKind::Imports, "Vapor");
    fixture.assert_reference(EdgeKind::Extends, "RouteCollection");
    fixture.assert_reference(EdgeKind::Implements, "Routable");
    fixture.assert_reference(EdgeKind::Extends, "View");
    fixture.assert_reference(EdgeKind::Calls, "routes.get");
    fixture.assert_reference(EdgeKind::Calls, "renderPayment");

    let boot = fixture
        .result()
        .nodes
        .iter()
        .find(|node| node.kind == NodeKind::Method && node.name == "boot")
        .unwrap();
    assert!(boot.is_static);
    assert!(boot.is_async);
    assert_eq!(boot.visibility.as_deref(), Some("public"));
    assert_eq!(boot.qualified_name, "PaymentsController.boot");
}

#[test]
fn harness_extracts_dart_symbols_imports_and_calls() {
    let fixture = OriginalSourceFixture::new(
        "payments.dart",
        r#"
import 'dart:async';
import 'package:flutter/widgets.dart' as widgets;

class PaymentWidget extends StatelessWidget {
  static Future<void> load() async {
    runApp(PaymentWidget());
  }
}

mixin Trackable {}

extension PaymentText on String {
  String label() {
    return trim();
  }
}

enum PaymentStatus { pending, paid }

typedef Handler = Future<void> Function();

Future<void> bootstrap() async {
  PaymentWidget.load();
}
"#,
    );

    assert_eq!(fixture.language(), Language::Dart);
    fixture.assert_node(NodeKind::Import, "dart:async");
    fixture.assert_node(NodeKind::Import, "package:flutter/widgets.dart");
    fixture.assert_node(NodeKind::Class, "PaymentWidget");
    fixture.assert_node(NodeKind::Trait, "Trackable");
    fixture.assert_node(NodeKind::Trait, "PaymentText");
    fixture.assert_node(NodeKind::Enum, "PaymentStatus");
    fixture.assert_node(NodeKind::TypeAlias, "Handler");
    fixture.assert_node(NodeKind::Method, "load");
    fixture.assert_node(NodeKind::Function, "bootstrap");
    fixture.assert_reference(EdgeKind::Imports, "dart:async");
    fixture.assert_reference(EdgeKind::Imports, "package:flutter/widgets.dart");
    fixture.assert_reference(EdgeKind::Extends, "StatelessWidget");
    fixture.assert_reference(EdgeKind::Calls, "runApp");
    fixture.assert_reference(EdgeKind::Calls, "PaymentWidget.load");

    let load = fixture
        .result()
        .nodes
        .iter()
        .find(|node| node.kind == NodeKind::Method && node.name == "load")
        .unwrap();
    assert!(load.is_static);
    assert!(load.is_async);
    assert_eq!(load.qualified_name, "PaymentWidget.load");
}

#[test]
fn harness_extracts_pascal_unit_class_functions_uses_and_calls() {
    let fixture = OriginalSourceFixture::new(
        "Payments.pas",
        r#"
unit Payments;

interface

uses SysUtils, Classes;

type
  TPaymentForm = class(TForm)
  public
    class procedure Register;
    function Total: Integer;
  end;

procedure BootstrapPayments;

implementation

procedure BootstrapPayments;
begin
  TPaymentForm.Register;
end;

end.
"#,
    );

    assert_eq!(fixture.language(), Language::Pascal);
    fixture.assert_node(NodeKind::Module, "Payments");
    fixture.assert_node(NodeKind::Import, "SysUtils");
    fixture.assert_node(NodeKind::Import, "Classes");
    fixture.assert_node(NodeKind::Class, "TPaymentForm");
    fixture.assert_node(NodeKind::Method, "Register");
    fixture.assert_node(NodeKind::Method, "Total");
    fixture.assert_node(NodeKind::Function, "BootstrapPayments");
    fixture.assert_reference(EdgeKind::Imports, "SysUtils");
    fixture.assert_reference(EdgeKind::Imports, "Classes");
    fixture.assert_reference(EdgeKind::Extends, "TForm");
    fixture.assert_reference(EdgeKind::Calls, "TPaymentForm.Register");
}

#[test]
fn harness_extracts_scala_symbols_imports_inheritance_and_calls() {
    let fixture = OriginalSourceFixture::new(
        "Payments.scala",
        r#"
import scala.concurrent.Future

trait Routable {
  def routes(): Unit
}

class PaymentController extends BaseController with Routable {
  def index(id: String): Future[String] = {
    Future.successful(renderPayment(id))
  }
}

object PaymentApp {
  def bootstrap(): Unit = {
    PaymentController()
  }
}

type Handler = String => String
"#,
    );

    assert_eq!(fixture.language(), Language::Scala);
    fixture.assert_node(NodeKind::Import, "scala.concurrent.Future");
    fixture.assert_node(NodeKind::Trait, "Routable");
    fixture.assert_node(NodeKind::Class, "PaymentController");
    fixture.assert_node(NodeKind::Module, "PaymentApp");
    fixture.assert_node(NodeKind::Method, "index");
    fixture.assert_node(NodeKind::Method, "bootstrap");
    fixture.assert_node(NodeKind::TypeAlias, "Handler");
    fixture.assert_reference(EdgeKind::Imports, "scala.concurrent.Future");
    fixture.assert_reference(EdgeKind::Extends, "BaseController");
    fixture.assert_reference(EdgeKind::Implements, "Routable");
    fixture.assert_reference(EdgeKind::Calls, "Future.successful");
    fixture.assert_reference(EdgeKind::Calls, "renderPayment");
}

#[test]
fn harness_extracts_liquid_template_references_schema_and_assignments() {
    let fixture = OriginalSourceFixture::new(
        "templates/product.liquid",
        r#"
{% assign product_title = product.title %}
{% render 'price-card', product: product %}
{% include "promo-banner" %}
{% section 'featured-products' %}

{% schema %}
{ "name": "Product template" }
{% endschema %}
"#,
    );

    assert_eq!(fixture.language(), Language::Liquid);
    fixture.assert_node(NodeKind::Variable, "product_title");
    fixture.assert_node(NodeKind::Import, "price-card");
    fixture.assert_node(NodeKind::Import, "promo-banner");
    fixture.assert_node(NodeKind::Import, "featured-products");
    fixture.assert_node(NodeKind::Component, "price-card");
    fixture.assert_node(NodeKind::Component, "featured-products");
    fixture.assert_node(NodeKind::Constant, "schema");
    fixture.assert_reference(EdgeKind::References, "snippets/price-card.liquid");
    fixture.assert_reference(EdgeKind::References, "snippets/promo-banner.liquid");
    fixture.assert_reference(EdgeKind::References, "sections/featured-products.liquid");
}

#[test]
fn harness_extracts_vue_component_script_symbols_and_template_refs() {
    let fixture = OriginalSourceFixture::new(
        "src/components/PaymentPanel.vue",
        r#"
<template>
  <PaymentSummary :total="total" />
</template>

<script setup lang="ts">
import PaymentSummary from './PaymentSummary.vue';

export const total = () => calculateTotal();

function calculateTotal(): number {
  return 42;
}
</script>
"#,
    );

    assert_eq!(fixture.language(), Language::Vue);
    fixture.assert_exported_node(NodeKind::Component, "PaymentPanel");
    fixture.assert_node(NodeKind::Import, "./PaymentSummary.vue");
    fixture.assert_exported_node(NodeKind::Function, "total");
    fixture.assert_node(NodeKind::Function, "calculateTotal");
    fixture.assert_reference(EdgeKind::Imports, "./PaymentSummary.vue");
    fixture.assert_reference(EdgeKind::References, "PaymentSummary");
    fixture.assert_reference(EdgeKind::Calls, "calculateTotal");
}

#[test]
fn harness_extracts_svelte_component_script_symbols_template_calls_and_components() {
    let fixture = OriginalSourceFixture::new(
        "src/routes/Checkout.svelte",
        r#"
<script lang="ts">
import CartSummary from './CartSummary.svelte';

export function submitPayment() {
  return completeCheckout();
}
</script>

<CartSummary />
<button class={buttonVariants({ size: 'sm' })} on:click={submitPayment}>Pay</button>
<p>{$state('ignored-rune')}</p>
"#,
    );

    assert_eq!(fixture.language(), Language::Svelte);
    fixture.assert_exported_node(NodeKind::Component, "Checkout");
    fixture.assert_node(NodeKind::Import, "./CartSummary.svelte");
    fixture.assert_exported_node(NodeKind::Function, "submitPayment");
    fixture.assert_reference(EdgeKind::Imports, "./CartSummary.svelte");
    fixture.assert_reference(EdgeKind::References, "CartSummary");
    fixture.assert_reference(EdgeKind::Calls, "completeCheckout");
    fixture.assert_reference(EdgeKind::Calls, "buttonVariants");
    assert!(
        !fixture
            .result()
            .unresolved_references
            .iter()
            .any(|reference| reference.reference_name == "$state"),
        "Svelte rune calls should not be captured as unresolved references"
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
fn harness_resolves_relative_imports_to_indexed_files() {
    let project = OriginalFixtureProject::new(&[
        (
            "src/payment.ts",
            r#"
import { charge } from './stripe';

export function processPayment() {
  return charge();
}
"#,
        ),
        (
            "src/stripe.ts",
            r#"
export function charge() {
  return true;
}
"#,
        ),
    ]);

    let cg = project.index();
    let dependents = cg.get_file_dependents("src/stripe.ts").unwrap();
    assert_eq!(dependents, vec!["src/payment.ts"]);
}

#[test]
fn harness_resolves_tsconfig_path_alias_imports_to_indexed_files() {
    let project = OriginalFixtureProject::new(&[
        (
            "tsconfig.json",
            r#"
{
  "compilerOptions": {
    "baseUrl": ".",
    "paths": {
      "@lib/*": ["src/lib/*"]
    }
  }
}
"#,
        ),
        (
            "src/app.ts",
            r#"
import { formatReceipt } from '@lib/receipt';

export function render() {
  return formatReceipt();
}
"#,
        ),
        (
            "src/lib/receipt.ts",
            r#"
export function formatReceipt() {
  return 'ok';
}
"#,
        ),
    ]);

    let cg = project.index();
    let dependents = cg.get_file_dependents("src/lib/receipt.ts").unwrap();
    assert_eq!(dependents, vec!["src/app.ts"]);
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
