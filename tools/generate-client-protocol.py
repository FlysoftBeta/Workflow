#!/usr/bin/env python3
"""Generate lossless Kotlin views of the Server-exported JSON Schema and method catalog.

Objects expose typed fields while retaining the original JSON for exact retransmission, including
absent/null distinctions and unknown additions. Only the explicitly opaque schema is untyped.
"""
import argparse
import hashlib
import json
import re
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
DEST = ROOT / 'app/client/src/main/kotlin/top/flysoftbeta/workflow/client/protocol/EngineBindings.kt'
SCHEMA = ROOT / 'engine/protocol/schema.json'
CONTRACT = ROOT / 'engine/protocol/contract.json'


def generate():
    schema = json.loads(SCHEMA.read_text())
    contract = json.loads(CONTRACT.read_text())
    declarations = {}
    pending = dict(schema['$defs'])
    def name(s):
        return re.sub(r'[^A-Za-z0-9_]', '_', s[:1].upper() + s[1:])
    def literal(value):
        return json.dumps(value, ensure_ascii=True).replace('$', '\\$')
    def expr(spec, value, hint):
        if spec is True or spec == {}:
            return 'OpaqueJson', f'OpaqueJson({value})'
        if spec is False:
            return 'Nothing', f'error("Schema forbids this value")'
        if '$ref' in spec:
            target = spec['$ref'].rsplit('/', 1)[-1]
            return target, f'{target}.decode({value})'
        variants = spec.get('anyOf') or spec.get('oneOf')
        types = spec.get('type')
        if variants:
            nonnull = [v for v in variants if v != {'type': 'null'}]
            if len(nonnull) == 1 and len(nonnull) != len(variants):
                t, e = expr(nonnull[0], 'it', hint)
                return t + '?', f'Wire.nullable({value}) {{ {e} }}'
            pending.setdefault(hint, spec)
            return hint, f'{hint}.decode({value})'
        if isinstance(types, list):
            nonnull = [v for v in types if v != 'null']
            if len(nonnull) == 1:
                t, e = expr(dict(spec, type=nonnull[0]), 'it', hint)
                return t + '?', f'Wire.nullable({value}) {{ {e} }}'
        if types == 'object':
            if not spec.get('properties') and isinstance(spec.get('additionalProperties'), dict):
                t, e = expr(spec['additionalProperties'], 'it', hint + 'Entry')
                return f'Map<String, {t}>', f'Wire.obj({value}).mapValues {{ (_, it) -> {e} }}'
            pending.setdefault(hint, spec)
            return hint, f'{hint}.decode({value})'
        if types == 'array':
            t, e = expr(spec.get('items', True), 'it', hint + 'Item')
            code = f'Wire.array({value}).map {{ {e} }}'
            checks = []
            if 'minItems' in spec: checks.append(f'it.size >= {spec["minItems"]}')
            if 'maxItems' in spec: checks.append(f'it.size <= {spec["maxItems"]}')
            if checks: code += ' .also { require(' + ' && '.join(checks) + ') { "Invalid array length" } }'
            return f'List<{t}>', code
        f = {'string': ('String', 'string'), 'integer': ('BigInteger', 'integer'),
             'number': ('BigDecimal', 'number'), 'boolean': ('Boolean', 'boolean'),
             'null': ('JsonNull', 'nil')}.get(types)
        if f:
            t, fn = f
            code = f'Wire.{fn}({value})'
            checks = []
            for bound, operator in [('minimum', '>='), ('maximum', '<=')]:
                if bound in spec:
                    checks.append(f'it {operator} {t}({literal(str(spec[bound]))})')
            if 'pattern' in spec:
                checks.append('Regex(' + literal(spec['pattern']) + ').containsMatchIn(it)')
            if 'enum' in spec:
                checks.append('it in setOf(' + ', '.join(literal(x) for x in spec['enum']) + ')')
            if checks:
                code += ' .also { require(' + ' && '.join(checks) + ') { "Value outside the Engine schema" } }'
            return t, code
        raise ValueError((hint, spec))

    while pending:
        key = next(iter(pending))
        spec = pending.pop(key)
        if key in declarations:
            continue
        declarations[key] = ''
        if key == 'OpaqueJson':
            declarations[key] = 'class OpaqueJson(override val json: JsonElement) : WireValue { companion object { fun decode(value: JsonElement) = OpaqueJson(value) } }'
            continue
        variants = spec.get('oneOf', spec.get('anyOf')) if isinstance(spec, dict) else None
        lines = [f'class {key} private constructor(override val json: JsonElement) : WireValue {{']
        if variants:
            alternatives = []
            for i, variant in enumerate(variants):
                tag = next((p['const'] for p in variant.get('properties', {}).values() if 'const' in p), None)
                label = name(str(tag or variant.get('$ref', '').rsplit('/', 1)[-1] or variant.get('type') or f'case{i}'))
                hint = key + label
                typ, decode = expr(variant, 'json', hint)
                # Kotlin's nullable alternative is still represented by the original JSON null.
                getter = f'as{label}{i}'
                lines.append(f'    val {getter}: {typ}? get() = Wire.attempt {{ {decode} }}')
                alternatives.append(f'Wire.matches {{ {decode} }}')
            check = ' + '.join(f'({c}).let {{ if (it) 1 else 0 }}' for c in alternatives)
            condition = '== 1' if 'oneOf' in spec else '> 0'
            lines.append(f'    init {{ require(({check}) {condition}) {{ "Invalid {key} variant" }} }}')
        elif isinstance(spec, dict) and spec.get('type') == 'object':
            lines.append('    private val fields = Wire.obj(json)')
            required = spec.get('required', [])
            for field, prop in spec.get('properties', {}).items():
                hint = key + name(field)
                typ, decode = expr(prop, 'value', hint)
                default = prop.get('default', None)
                if field not in required and 'default' not in prop:
                    value = f'Wire.optional(fields, {literal(field)})'
                    lines.append(f'    val `{field}`: {typ.rstrip("?")}? = {value}?.let {{ value -> {decode} }}')
                else:
                    if 'default' in prop:
                        fallback = f', {literal(json.dumps(default, ensure_ascii=True, separators=(",", ":")))}'
                    else:
                        fallback = ''
                    lines.append(f'    val `{field}`: {typ} = Wire.member(fields, {literal(field)}{fallback}).let {{ value -> {decode} }}')
                if 'const' in prop:
                    lines.append(f'    init {{ require(`{field}` == {literal(prop["const"])}) {{ "Invalid {key}.{field}" }} }}')
            # Preserve and type map contents as well as all future fields without discarding anything.
            if isinstance(spec.get('additionalProperties'), dict):
                typ, decode = expr(spec['additionalProperties'], 'value', key + 'Extra')
                known = ', '.join(literal(x) for x in spec.get('properties', {}))
                lines.append(f'    val additional: Map<String, {typ}> = fields.filterKeys {{ it !in setOf<String>({known}) }}.mapValues {{ (_, value) -> {decode} }}')
        else:
            typ, decode = expr(spec, 'json', key + 'Value')
            lines.append(f'    val value: {typ} = {decode}')
            if isinstance(spec, dict) and 'enum' in spec:
                options = ', '.join(literal(x) for x in spec['enum'])
                lines.append(f'    init {{ require(value in setOf({options})) {{ "Invalid {key}" }} }}')
        lines.append(f'    companion object {{ fun decode(value: JsonElement) = {key}(value) }}')
        lines.append('}')
        declarations[key] = '\n'.join(lines)
    digest = hashlib.sha256(SCHEMA.read_bytes() + CONTRACT.read_bytes()).hexdigest()
    output = ['// Generated by tools/generate-client-protocol.py; edit the Rust contract, then regenerate.',
              'package top.flysoftbeta.workflow.client.protocol', '',
              'import java.math.BigInteger', 'import java.math.BigDecimal', 'import kotlinx.serialization.json.*', '',
              f'const val ENGINE_CONTRACT_SHA256 = "{digest}"', '']
    output += [declarations[x] + '\n' for x in sorted(declarations)]
    output += ['object EngineMethods {']
    for method in contract['methods']:
        entry = contract['methodSchemas'][method]
        p = entry['params']['$ref'].rsplit('/', 1)[-1]
        r = entry['result']['$ref'].rsplit('/', 1)[-1]
        ident = method.replace('.', '_')
        output += [f'    val {ident} = RpcMethod({literal(method)}, {p}::decode, {r}::decode)']
    output += ['    val all: List<RpcMethod<*, *>> = listOf(']
    output += ['        ' + m.replace('.', '_') + ',' for m in contract['methods']]
    output += ['    )', '}']
    return '\n'.join(output) + '\n'


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--check', action='store_true')
    args = parser.parse_args()
    expected = generate()
    if args.check:
        if not DEST.exists() or DEST.read_text() != expected:
            raise SystemExit('Kotlin bindings drifted; run tools/generate-client-protocol.py')
        print('Kotlin protocol bindings match the Rust export')
    else:
        DEST.parent.mkdir(parents=True, exist_ok=True)
        DEST.write_text(expected)

if __name__ == '__main__':
    main()
