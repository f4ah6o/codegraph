/**
 * Import Resolver
 *
 * Resolves import paths to actual files and symbols.
 */

import * as path from 'path';
import type { Node as SyntaxNode, Tree } from 'web-tree-sitter';
import { Language, Node } from '../types';
import { UnresolvedRef, ResolvedRef, ResolutionContext, ImportMapping, ReExport } from './types';
import { applyAliases } from './path-aliases';
import { getParser } from '../extraction/grammars';
import { getChildByField, getNodeText } from '../extraction/tree-sitter-helpers';

/**
 * Extension resolution order by language
 */
const EXTENSION_RESOLUTION: Record<string, string[]> = {
  typescript: ['.ts', '.tsx', '.d.ts', '.js', '.jsx', '/index.ts', '/index.tsx', '/index.js'],
  javascript: ['.js', '.jsx', '.mjs', '.cjs', '/index.js', '/index.jsx'],
  tsx: ['.tsx', '.ts', '.d.ts', '.js', '.jsx', '/index.tsx', '/index.ts', '/index.js'],
  jsx: ['.jsx', '.js', '/index.jsx', '/index.js'],
  python: ['.py', '/__init__.py'],
  go: ['.go'],
  rust: ['.rs', '/mod.rs'],
  java: ['.java'],
  csharp: ['.cs'],
  php: ['.php'],
  ruby: ['.rb'],
};

/**
 * Resolve an import path to an actual file
 */
export function resolveImportPath(
  importPath: string,
  fromFile: string,
  language: Language,
  context: ResolutionContext
): string | null {
  // Skip external/npm packages — but pass the context so the
  // bare-specifier heuristic can consult the project's tsconfig
  // alias map first (custom prefixes like `@components/*` would
  // otherwise be misclassified as npm).
  if (isExternalImport(importPath, language, context)) {
    return null;
  }

  const projectRoot = context.getProjectRoot();
  const fromDir = path.dirname(path.join(projectRoot, fromFile));

  // Handle relative imports
  if (importPath.startsWith('.')) {
    return resolveRelativeImport(importPath, fromDir, language, context);
  }

  // Handle absolute/aliased imports (like @/ or src/)
  return resolveAliasedImport(importPath, projectRoot, language, context);
}

/**
 * Check if an import is external (npm package, etc.)
 *
 * `context` is consulted for project-defined path aliases
 * (tsconfig/jsconfig `paths`). Without that check, custom prefixes
 * like `@components/*` would fail the bare-specifier heuristic and
 * be classified as external before alias resolution can run.
 */
function isExternalImport(
  importPath: string,
  language: Language,
  context?: ResolutionContext
): boolean {
  // Relative imports are not external
  if (importPath.startsWith('.')) {
    return false;
  }

  // Common external patterns
  if (language === 'typescript' || language === 'javascript' || language === 'tsx' || language === 'jsx') {
    // Node built-ins
    if (['fs', 'path', 'os', 'crypto', 'http', 'https', 'url', 'util', 'events', 'stream', 'child_process', 'buffer'].includes(importPath)) {
      return true;
    }
    // Project-defined alias prefix? Treat as local.
    const aliases = context?.getProjectAliases?.();
    if (aliases) {
      for (const pat of aliases.patterns) {
        if (importPath.startsWith(pat.prefix)) return false;
      }
    }
    // Scoped packages or bare specifiers that don't start with aliases
    if (!importPath.startsWith('@/') && !importPath.startsWith('~/') && !importPath.startsWith('src/')) {
      // Likely an npm package
      return true;
    }
  }

  if (language === 'python') {
    // Standard library modules
    const stdLibs = ['os', 'sys', 'json', 're', 'math', 'datetime', 'collections', 'typing', 'pathlib', 'logging'];
    if (stdLibs.includes(importPath.split('.')[0]!)) {
      return true;
    }
  }

  if (language === 'go') {
    // Standard library or external packages
    if (!importPath.startsWith('.') && !importPath.includes('/internal/')) {
      return true;
    }
  }

  return false;
}

/**
 * Resolve a relative import
 */
function resolveRelativeImport(
  importPath: string,
  fromDir: string,
  language: Language,
  context: ResolutionContext
): string | null {
  const projectRoot = context.getProjectRoot();
  const extensions = EXTENSION_RESOLUTION[language] || [];

  // Try the path as-is first
  const basePath = path.resolve(fromDir, importPath);
  const relativePath = path.relative(projectRoot, basePath).replace(/\\/g, '/');

  // Try each extension
  for (const ext of extensions) {
    const candidatePath = relativePath + ext;
    if (context.fileExists(candidatePath)) {
      return candidatePath;
    }
  }

  // Try without extension (might already have one)
  if (context.fileExists(relativePath)) {
    return relativePath;
  }

  return null;
}

/**
 * Resolve an aliased/absolute import.
 *
 * Tries, in order:
 *   1. Project-defined `compilerOptions.paths` (tsconfig/jsconfig).
 *      Each pattern can have multiple replacements; tried in tsconfig
 *      priority order with extension permutations.
 *   2. The legacy hard-coded fallback list (`@/`, `~/`, `src/`, ...)
 *      for projects that have aliases but no tsconfig paths block.
 *   3. Direct path lookup (with extensions).
 */
function resolveAliasedImport(
  importPath: string,
  projectRoot: string,
  language: Language,
  context: ResolutionContext
): string | null {
  const extensions = EXTENSION_RESOLUTION[language] || [];
  const tryWithExt = (basePath: string): string | null => {
    for (const ext of extensions) {
      const candidate = basePath + ext;
      if (context.fileExists(candidate)) return candidate;
    }
    if (context.fileExists(basePath)) return basePath;
    return null;
  };

  // 1. Project tsconfig/jsconfig paths.
  const aliasMap = context.getProjectAliases?.();
  if (aliasMap) {
    const candidates = applyAliases(importPath, aliasMap, projectRoot);
    for (const c of candidates) {
      const hit = tryWithExt(c);
      if (hit) return hit;
    }
  }

  // 2. Hard-coded fallback list. Kept for projects that use these
  //    conventional aliases without declaring them in tsconfig.
  const fallbackAliases: Record<string, string> = {
    '@/': 'src/',
    '~/': 'src/',
    '@src/': 'src/',
    'src/': 'src/',
    '@app/': 'app/',
    'app/': 'app/',
  };
  for (const [alias, replacement] of Object.entries(fallbackAliases)) {
    if (importPath.startsWith(alias)) {
      const hit = tryWithExt(importPath.replace(alias, replacement));
      if (hit) return hit;
    }
  }

  // 3. Direct path.
  return tryWithExt(importPath);
}

/**
 * Extract import mappings from a file
 */
export function extractImportMappings(
  _filePath: string,
  content: string,
  language: Language
): ImportMapping[] {
  const mappings: ImportMapping[] = [];

  if (language === 'typescript' || language === 'javascript' || language === 'tsx' || language === 'jsx') {
    mappings.push(...extractJSImports(content, language));
  } else if (language === 'python') {
    mappings.push(...extractPythonImports(content));
  } else if (language === 'go') {
    mappings.push(...extractGoImports(content));
  } else if (language === 'php') {
    mappings.push(...extractPHPImports(content));
  }

  return mappings;
}

/**
 * Extract JS/TS import mappings
 *
 * Uses tree-sitter when the JS/TS grammar is available, with the old
 * regex scanner retained as a fallback for callers that use this module
 * before grammar initialization.
 */
function extractJSImports(content: string, language: Language): ImportMapping[] {
  return extractJSImportsWithTreeSitter(content, language) ?? extractJSImportsRegex(content);
}

function extractJSImportsWithTreeSitter(content: string, language: Language): ImportMapping[] | null {
  const parsed = parseJsTs(content, language);
  if (!parsed) return null;

  const mappings: ImportMapping[] = [];
  try {
    walkNamed(parsed.tree.rootNode, (node) => {
      if (node.type === 'import_statement') {
        mappings.push(...extractImportStatementMappings(node, content));
        return false;
      }
      if (node.type === 'variable_declarator') {
        const requireSource = getRequireSource(node, content);
        if (requireSource) {
          mappings.push(...extractRequireMappings(node, content, requireSource));
          return false;
        }
      }
      return true;
    });
  } finally {
    parsed.tree.delete();
  }

  return mappings;
}

function extractJSImportsRegex(content: string): ImportMapping[] {
  const mappings: ImportMapping[] = [];

  // ES6 imports
  const importRegex = /import\s+(?:(\w+)\s*,?\s*)?(?:\{([^}]+)\})?\s*(?:(\*)\s+as\s+(\w+))?\s*from\s*['"]([^'"]+)['"]/g;

  let match;
  while ((match = importRegex.exec(content)) !== null) {
    const [, defaultImport, namedImports, star, namespaceAlias, source] = match;

    // Default import
    if (defaultImport) {
      mappings.push({
        localName: defaultImport,
        exportedName: 'default',
        source: source!,
        isDefault: true,
        isNamespace: false,
      });
    }

    // Named imports
    if (namedImports) {
      const names = namedImports.split(',').map((s) => s.trim());
      for (const name of names) {
        const aliasMatch = name.match(/(\w+)\s+as\s+(\w+)/);
        if (aliasMatch) {
          mappings.push({
            localName: aliasMatch[2]!,
            exportedName: aliasMatch[1]!,
            source: source!,
            isDefault: false,
            isNamespace: false,
          });
        } else if (name) {
          mappings.push({
            localName: name,
            exportedName: name,
            source: source!,
            isDefault: false,
            isNamespace: false,
          });
        }
      }
    }

    // Namespace import
    if (star && namespaceAlias) {
      mappings.push({
        localName: namespaceAlias,
        exportedName: '*',
        source: source!,
        isDefault: false,
        isNamespace: true,
      });
    }
  }

  // Require statements
  const requireRegex = /(?:const|let|var)\s+(?:(\w+)|{([^}]+)})\s*=\s*require\(['"]([^'"]+)['"]\)/g;
  while ((match = requireRegex.exec(content)) !== null) {
    const [, defaultName, destructured, source] = match;

    if (defaultName) {
      mappings.push({
        localName: defaultName,
        exportedName: 'default',
        source: source!,
        isDefault: true,
        isNamespace: false,
      });
    }

    if (destructured) {
      const names = destructured.split(',').map((s) => s.trim());
      for (const name of names) {
        const aliasMatch = name.match(/(\w+)\s*:\s*(\w+)/);
        if (aliasMatch) {
          mappings.push({
            localName: aliasMatch[2]!,
            exportedName: aliasMatch[1]!,
            source: source!,
            isDefault: false,
            isNamespace: false,
          });
        } else if (name) {
          mappings.push({
            localName: name,
            exportedName: name,
            source: source!,
            isDefault: false,
            isNamespace: false,
          });
        }
      }
    }
  }

  return mappings;
}

function parseJsTs(content: string, language: Language): { tree: Tree } | null {
  if (!isJsTsLanguage(language)) return null;
  const parser = getParser(language) ?? getParser(language === 'tsx' ? 'typescript' : language === 'jsx' ? 'javascript' : language);
  if (!parser) return null;
  const tree = parser.parse(content);
  return tree ? { tree } : null;
}

function isJsTsLanguage(language: Language): boolean {
  return language === 'typescript' || language === 'javascript' || language === 'tsx' || language === 'jsx';
}

function walkNamed(node: SyntaxNode, visit: (node: SyntaxNode) => boolean): void {
  const shouldDescend = visit(node);
  if (!shouldDescend) return;
  for (let i = 0; i < node.namedChildCount; i++) {
    const child = node.namedChild(i);
    if (child) walkNamed(child, visit);
  }
}

function extractImportStatementMappings(node: SyntaxNode, source: string): ImportMapping[] {
  const moduleName = getStringLiteralValue(getChildByField(node, 'source') ?? findLastStringChild(node), source);
  if (!moduleName) return [];

  const mappings: ImportMapping[] = [];

  walkNamed(node, (child) => {
    if (child.type === 'import_specifier') {
      const mapping = importSpecifierToMapping(child, source, moduleName);
      if (mapping) mappings.push(mapping);
      return false;
    }
    if (child.type === 'namespace_import') {
      const local = identifierTexts(child, source).pop();
      if (local) {
        mappings.push({
          localName: local,
          exportedName: '*',
          source: moduleName,
          isDefault: false,
          isNamespace: true,
        });
      }
      return false;
    }
    return true;
  });

  const importClause = node.namedChildren.find((child) => child.type === 'import_clause');
  if (importClause) {
    for (const id of importClause.namedChildren) {
      if (id.type === 'identifier') {
        mappings.push({
          localName: getNodeText(id, source),
          exportedName: 'default',
          source: moduleName,
          isDefault: true,
          isNamespace: false,
        });
      }
    }
  }

  return dedupeImportMappings(mappings);
}

function importSpecifierToMapping(node: SyntaxNode, source: string, moduleName: string): ImportMapping | null {
  const text = getNodeText(node, source).replace(/^type\s+/, '').trim();
  const aliasMatch = text.match(/^([A-Za-z_$][\w$]*)\s+as\s+([A-Za-z_$][\w$]*)$/);
  if (aliasMatch) {
    return {
      localName: aliasMatch[2]!,
      exportedName: aliasMatch[1]!,
      source: moduleName,
      isDefault: false,
      isNamespace: false,
    };
  }
  const ids = identifierTexts(node, source);
  const name = ids[ids.length - 1] ?? text;
  if (!name) return null;
  return {
    localName: name,
    exportedName: name,
    source: moduleName,
    isDefault: false,
    isNamespace: false,
  };
}

function getRequireSource(node: SyntaxNode, source: string): string | null {
  const value = getChildByField(node, 'value');
  if (!value || value.type !== 'call_expression') return null;
  const fn = getChildByField(value, 'function') ?? value.namedChild(0);
  if (!fn || getNodeText(fn, source) !== 'require') return null;
  const args = getChildByField(value, 'arguments');
  const literal = args?.namedChildren.find((child) => child.type === 'string');
  return getStringLiteralValue(literal ?? null, source);
}

function extractRequireMappings(node: SyntaxNode, source: string, moduleName: string): ImportMapping[] {
  const nameNode = getChildByField(node, 'name');
  if (!nameNode) return [];

  if (nameNode.type === 'identifier') {
    return [{
      localName: getNodeText(nameNode, source),
      exportedName: 'default',
      source: moduleName,
      isDefault: true,
      isNamespace: false,
    }];
  }

  if (nameNode.type === 'object_pattern') {
    return parseObjectPatternNames(getNodeText(nameNode, source)).map(({ exportedName, localName }) => ({
      localName,
      exportedName,
      source: moduleName,
      isDefault: false,
      isNamespace: false,
    }));
  }

  return [];
}

function parseObjectPatternNames(text: string): Array<{ exportedName: string; localName: string }> {
  const inner = text.replace(/^\{/, '').replace(/\}$/, '');
  return inner.split(',').map((raw) => raw.trim()).filter(Boolean).flatMap((item) => {
    const alias = item.match(/^([A-Za-z_$][\w$]*)\s*:\s*([A-Za-z_$][\w$]*)$/);
    if (alias) return [{ exportedName: alias[1]!, localName: alias[2]! }];
    const name = item.match(/^([A-Za-z_$][\w$]*)$/)?.[1];
    return name ? [{ exportedName: name, localName: name }] : [];
  });
}

function identifierTexts(node: SyntaxNode, source: string): string[] {
  const out: string[] = [];
  walkNamed(node, (child) => {
    if (child.type === 'identifier' || child.type === 'property_identifier') {
      out.push(getNodeText(child, source));
      return false;
    }
    return true;
  });
  return out;
}

function findLastStringChild(node: SyntaxNode): SyntaxNode | null {
  let found: SyntaxNode | null = null;
  walkNamed(node, (child) => {
    if (child.type === 'string') {
      found = child;
      return false;
    }
    return true;
  });
  return found;
}

function getStringLiteralValue(node: SyntaxNode | null, source: string): string | null {
  if (!node) return null;
  return getNodeText(node, source).replace(/^['"]|['"]$/g, '');
}

function dedupeImportMappings(mappings: ImportMapping[]): ImportMapping[] {
  const seen = new Set<string>();
  return mappings.filter((mapping) => {
    const key = `${mapping.localName}\0${mapping.exportedName}\0${mapping.source}\0${mapping.isDefault}\0${mapping.isNamespace}`;
    if (seen.has(key)) return false;
    seen.add(key);
    return true;
  });
}

/**
 * Extract Python import mappings
 */
function extractPythonImports(content: string): ImportMapping[] {
  const mappings: ImportMapping[] = [];

  // from X import Y
  const fromImportRegex = /from\s+([\w.]+)\s+import\s+([^#\n]+)/g;
  let match;

  while ((match = fromImportRegex.exec(content)) !== null) {
    const [, source, imports] = match;
    const names = imports!.split(',').map((s) => s.trim());

    for (const name of names) {
      const aliasMatch = name.match(/(\w+)\s+as\s+(\w+)/);
      if (aliasMatch) {
        mappings.push({
          localName: aliasMatch[2]!,
          exportedName: aliasMatch[1]!,
          source: source!,
          isDefault: false,
          isNamespace: false,
        });
      } else if (name && name !== '*') {
        mappings.push({
          localName: name,
          exportedName: name,
          source: source!,
          isDefault: false,
          isNamespace: false,
        });
      }
    }
  }

  // import X
  const importRegex = /^import\s+([\w.]+)(?:\s+as\s+(\w+))?/gm;
  while ((match = importRegex.exec(content)) !== null) {
    const [, source, alias] = match;
    const localName = alias || source!.split('.').pop()!;
    mappings.push({
      localName,
      exportedName: '*',
      source: source!,
      isDefault: false,
      isNamespace: true,
    });
  }

  return mappings;
}

/**
 * Extract Go import mappings
 */
function extractGoImports(content: string): ImportMapping[] {
  const mappings: ImportMapping[] = [];

  // import "path" or import alias "path"
  const singleImportRegex = /import\s+(?:(\w+)\s+)?["']([^"']+)["']/g;
  let match;

  while ((match = singleImportRegex.exec(content)) !== null) {
    const [, alias, source] = match;
    const packageName = source!.split('/').pop()!;
    mappings.push({
      localName: alias || packageName,
      exportedName: '*',
      source: source!,
      isDefault: false,
      isNamespace: true,
    });
  }

  // import ( ... ) block
  const blockImportRegex = /import\s*\(\s*([^)]+)\s*\)/gs;
  while ((match = blockImportRegex.exec(content)) !== null) {
    const block = match[1]!;
    const lineRegex = /(?:(\w+)\s+)?["']([^"']+)["']/g;
    let lineMatch;

    while ((lineMatch = lineRegex.exec(block)) !== null) {
      const [, alias, source] = lineMatch;
      const packageName = source!.split('/').pop()!;
      mappings.push({
        localName: alias || packageName,
        exportedName: '*',
        source: source!,
        isDefault: false,
        isNamespace: true,
      });
    }
  }

  return mappings;
}

/**
 * Extract PHP import mappings (use statements)
 */
function extractPHPImports(content: string): ImportMapping[] {
  const mappings: ImportMapping[] = [];

  // use Namespace\Class; or use Namespace\Class as Alias;
  const useRegex = /use\s+([\w\\]+)(?:\s+as\s+(\w+))?;/g;
  let match;

  while ((match = useRegex.exec(content)) !== null) {
    const [, fullPath, alias] = match;
    const className = fullPath!.split('\\').pop()!;
    mappings.push({
      localName: alias || className,
      exportedName: className,
      source: fullPath!,
      isDefault: false,
      isNamespace: false,
    });
  }

  return mappings;
}

// Cache import mappings per file to avoid re-reading and re-parsing
const importMappingCache = new Map<string, ImportMapping[]>();

/**
 * Clear the import mapping cache (call between indexing runs)
 */
export function clearImportMappingCache(): void {
  importMappingCache.clear();
}

/**
 * Strip JS line + block comments from `content` while preserving
 * string literals (so `"//"` inside a string stays intact). Used by
 * {@link extractReExports} so commented-out export-from statements
 * don't generate phantom re-export edges.
 *
 * Scanner is deliberately small: it only tracks the three contexts
 * relevant for JS/TS — single-quote string, double-quote string, and
 * template literal. Comment recognition is the JS spec subset, no
 * regex-literal awareness (which is fine for our use case: we don't
 * apply this to function bodies, only to top-level files).
 */
function stripJsComments(content: string): string {
  let out = '';
  let i = 0;
  let str: '"' | "'" | '`' | null = null;
  while (i < content.length) {
    const ch = content[i]!;
    if (str !== null) {
      out += ch;
      if (ch === '\\' && i + 1 < content.length) {
        out += content[i + 1]!;
        i += 2;
        continue;
      }
      if (ch === str) str = null;
      i++;
      continue;
    }
    if (ch === '"' || ch === "'" || ch === '`') {
      str = ch;
      out += ch;
      i++;
      continue;
    }
    if (ch === '/' && content[i + 1] === '/') {
      while (i < content.length && content[i] !== '\n') i++;
      continue;
    }
    if (ch === '/' && content[i + 1] === '*') {
      i += 2;
      while (i < content.length && !(content[i] === '*' && content[i + 1] === '/')) i++;
      i += 2;
      continue;
    }
    out += ch;
    i++;
  }
  return out;
}

/**
 * Extract JS/TS re-export declarations from `content`.
 *
 * Recognised forms:
 *   export { foo } from './a';
 *   export { foo as bar } from './a';
 *   export * from './a';
 *   export * as ns from './a';   (treated as wildcard for chasing)
 *   export { default as Foo } from './a';
 *
 * Uses tree-sitter when the JS/TS grammar is available, with the old
 * regex scanner retained as a fallback for callers that use this module
 * before grammar initialization.
 */
export function extractReExports(content: string, language: Language): ReExport[] {
  if (
    !isJsTsLanguage(language)
  ) {
    return [];
  }

  return extractReExportsWithTreeSitter(content, language) ?? extractReExportsRegex(content);
}

function extractReExportsWithTreeSitter(content: string, language: Language): ReExport[] | null {
  const parsed = parseJsTs(content, language);
  if (!parsed) return null;

  const out: ReExport[] = [];
  try {
    walkNamed(parsed.tree.rootNode, (node) => {
      if (node.type !== 'export_statement') return true;
      const source = getStringLiteralValue(getChildByField(node, 'source') ?? findLastStringChild(node), content);
      if (!source) return false;

      const text = getNodeText(node, content);
      if (/^export\s*\*/.test(text.trim())) {
        out.push({ kind: 'wildcard', source });
        return false;
      }

      const clause = node.namedChildren.find((child) =>
        child.type === 'export_clause' ||
        child.type === 'named_exports' ||
        child.type === 'export_specifier'
      );
      const clauseText = clause ? getNodeText(clause, content) : text;
      for (const spec of parseExportSpecifiers(clauseText)) {
        out.push({ ...spec, source });
      }
      return false;
    });
  } finally {
    parsed.tree.delete();
  }

  return out;
}

function parseExportSpecifiers(text: string): Array<Omit<Extract<ReExport, { kind: 'named' }>, 'source'>> {
  const match = text.match(/\{([\s\S]*)\}/);
  const inner = match ? match[1]! : text;
  return inner.split(',').map((raw) => raw.trim()).filter(Boolean).flatMap((item) => {
    const clean = item.replace(/^type\s+/, '').trim();
    const aliasMatch = clean.match(/^([A-Za-z_$][\w$]*|default)\s+as\s+([A-Za-z_$][\w$]*)$/);
    if (aliasMatch) {
      return [{
        kind: 'named' as const,
        exportedName: aliasMatch[2]!,
        originalName: aliasMatch[1]!,
      }];
    }
    const name = clean.match(/^([A-Za-z_$][\w$]*|default)$/)?.[1];
    return name ? [{
      kind: 'named' as const,
      exportedName: name,
      originalName: name,
    }] : [];
  });
}

function extractReExportsRegex(content: string): ReExport[] {
  const out: ReExport[] = [];

  // Pre-strip block comments + line comments so a commented-out
  // `// export { x } from '...'` doesn't produce a phantom edge.
  // (Template literals are still a possible source of false positives;
  // a project that builds export statements as runtime strings is
  // out of scope.)
  const cleaned = stripJsComments(content);

  // Wildcard: `export * from '...'` or `export * as ns from '...'`
  const wildcardRe = /export\s*\*(?:\s+as\s+\w+)?\s*from\s*['"]([^'"]+)['"]/g;
  let m: RegExpExecArray | null;
  while ((m = wildcardRe.exec(cleaned)) !== null) {
    out.push({ kind: 'wildcard', source: m[1]! });
  }

  // Named: `export { a, b as c } from '...'`
  const namedRe = /export\s*\{([^}]+)\}\s*from\s*['"]([^'"]+)['"]/g;
  while ((m = namedRe.exec(cleaned)) !== null) {
    const inner = m[1]!;
    const source = m[2]!;
    for (const raw of inner.split(',')) {
      const item = raw.trim();
      if (!item) continue;
      const aliasMatch = item.match(/^(\w+)\s+as\s+(\w+)$/);
      if (aliasMatch) {
        out.push({
          kind: 'named',
          exportedName: aliasMatch[2]!,
          originalName: aliasMatch[1]!,
          source,
        });
      } else if (/^\w+$/.test(item)) {
        out.push({
          kind: 'named',
          exportedName: item,
          originalName: item,
          source,
        });
      }
    }
  }

  return out;
}

/**
 * Resolve a reference using import mappings
 */
export function resolveViaImport(
  ref: UnresolvedRef,
  context: ResolutionContext
): ResolvedRef | null {
  // Use cached import mappings (avoids re-reading and re-parsing per ref)
  const imports = context.getImportMappings(ref.filePath, ref.language);
  if (imports.length === 0 && !context.readFile(ref.filePath)) {
    return null;
  }

  // Check if the reference name matches any import
  for (const imp of imports) {
    if (imp.localName === ref.referenceName || ref.referenceName.startsWith(imp.localName + '.')) {
      // Resolve the import path
      const resolvedPath = resolveImportPath(
        imp.source,
        ref.filePath,
        ref.language,
        context
      );

      if (resolvedPath) {
        const exportedName = imp.isDefault ? 'default' : imp.exportedName;
        const memberName = imp.isNamespace
          ? ref.referenceName.replace(imp.localName + '.', '')
          : null;

        const targetNode = findExportedSymbol(
          resolvedPath,
          { isDefault: imp.isDefault, isNamespace: imp.isNamespace, exportedName, memberName },
          ref.language,
          context,
          new Set()
        );

        if (targetNode) {
          return {
            original: ref,
            targetNodeId: targetNode.id,
            confidence: 0.9,
            resolvedBy: 'import',
          };
        }
      }
    }
  }

  return null;
}

/** Recursive depth cap for re-export chain following. Real codebases
 *  rarely chain barrels more than 2–3 deep; 8 is a generous safety
 *  net that still bounds worst-case work. */
const REEXPORT_MAX_DEPTH = 8;

/**
 * Find an exported symbol in `filePath`, following `export { x } from
 * './other'` and `export * from './other'` chains until the original
 * declaration is reached. Cycle-safe via the `visited` set.
 *
 * Without this, every barrel-style import (`import { Foo } from
 * './index'` where `index.ts` only re-exports) used to resolve to
 * nothing — the existing code only looked for declarations IN the
 * resolved file, not declarations the file forwarded.
 */
function findExportedSymbol(
  filePath: string,
  want: {
    isDefault: boolean;
    isNamespace: boolean;
    exportedName: string;
    memberName: string | null;
  },
  language: Language,
  context: ResolutionContext,
  visited: Set<string>,
  depth = 0
): Node | undefined {
  if (depth > REEXPORT_MAX_DEPTH) return undefined;
  if (visited.has(filePath)) return undefined;
  visited.add(filePath);

  const nodesInFile = context.getNodesInFile(filePath);

  // 1. Direct hit: the symbol is declared in this file.
  if (want.isDefault) {
    const direct = nodesInFile.find(
      (n) => n.isExported && (n.kind === 'function' || n.kind === 'class')
    );
    if (direct) return direct;
  } else if (want.isNamespace && want.memberName) {
    const direct = nodesInFile.find(
      (n) => n.name === want.memberName && n.isExported
    );
    if (direct) return direct;
  } else {
    const direct = nodesInFile.find(
      (n) => n.name === want.exportedName && n.isExported
    );
    if (direct) return direct;
  }

  // 2. Re-export hit: the file forwards the symbol to another module.
  const reExports = context.getReExports?.(filePath, language) ?? [];
  if (reExports.length === 0) return undefined;

  // Look for explicit `export { want } from './other'` (with optional rename).
  const targetName = want.isDefault ? 'default' : want.exportedName;
  for (const rex of reExports) {
    if (rex.kind === 'named' && rex.exportedName === targetName) {
      const next = resolveImportPath(rex.source, filePath, language, context);
      if (!next) continue;
      // After rename: `export { foo as bar } from './x'` — to chase
      // `bar`, we look for `foo` in `./x`.
      const chained = findExportedSymbol(
        next,
        {
          isDefault: rex.originalName === 'default',
          isNamespace: false,
          exportedName: rex.originalName,
          memberName: null,
        },
        language,
        context,
        visited,
        depth + 1
      );
      if (chained) return chained;
    }
  }

  // 3. Wildcard re-export: `export * from './other'` — try every
  //    forwarding source. This is the barrel-of-barrels case.
  for (const rex of reExports) {
    if (rex.kind === 'wildcard') {
      const next = resolveImportPath(rex.source, filePath, language, context);
      if (!next) continue;
      const chained = findExportedSymbol(next, want, language, context, visited, depth + 1);
      if (chained) return chained;
    }
  }

  return undefined;
}
