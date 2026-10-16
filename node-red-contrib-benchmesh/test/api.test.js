// Run: node --test node-red-contrib-benchmesh/test
const test = require('node:test');
const assert = require('node:assert');
const { resolveOperation, buildRequest, numericValue } = require('../nodes/shared/api');

test('editor operation ids map to verb + partial class operation', () => {
    assert.deepStrictEqual(resolveOperation('PSU', 'query_voltage'), { verb: 'GET', name: 'voltage' });
    assert.deepStrictEqual(resolveOperation('PSU', 'set_output'), { verb: 'POST', name: 'output' });
    assert.deepStrictEqual(resolveOperation('DMM', 'query_measurement'), { verb: 'GET', name: 'measurement' });
    assert.deepStrictEqual(resolveOperation('ELL', 'set_current'), { verb: 'POST', name: 'curr' });
    assert.deepStrictEqual(resolveOperation('ELL', 'set_power'), { verb: 'POST', name: 'pow' });
    assert.deepStrictEqual(resolveOperation('ELL', 'set_input'), { verb: 'POST', name: 'input' });
    assert.deepStrictEqual(resolveOperation('PSU', 'voltage'), { verb: 'GET', name: 'voltage' });
});

test('requests never double the prefix and encode values', () => {
    assert.deepStrictEqual(buildRequest('http://h:1/', 'PSU', 'psu-1', '1', 'query_voltage'),
        { method: 'GET', url: 'http://h:1/instruments/PSU/psu-1/1/voltage' });
    assert.deepStrictEqual(buildRequest('http://h:1', 'PSU', 'psu-1', 2, 'set_voltage', 5.5),
        { method: 'POST', url: 'http://h:1/instruments/PSU/psu-1/2/voltage/5.5' });
    assert.throws(() => buildRequest('http://h:1', 'PSU', 'psu-1', 1, 'set_output', ''));
});

test('numeric values from SI dicts, numbers and strings', () => {
    assert.strictEqual(numericValue({ si: { number: '988', symbol: 'm' }, sci: '9.88000E-01', val: '0.988' }), 0.988);
    assert.strictEqual(numericValue(10), 10);
    assert.strictEqual(numericValue('1.000'), 1);
    assert.ok(Number.isNaN(numericValue('ON')));
    assert.ok(Number.isNaN(numericValue(null)));
});

test('class nodes call the API with partial names and report numeric values', async () => {
    const http = require('node:http');
    const { registerClassNode } = require('../nodes/shared/classNode');
    const seen = [];
    const server = http.createServer((req, res) => {
        seen.push(`${req.method} ${req.url}`);
        if (req.method === 'POST') { res.statusCode = 204; return res.end(); }
        res.setHeader('content-type', 'application/json');
        res.end(JSON.stringify({ path: req.url, value: { si: { number: '4.94' }, sci: '4.94000E+00', val: '4.94' } }));
    });
    await new Promise((r) => server.listen(0, r));
    const base = `http://127.0.0.1:${server.address().port}`;
    let Ctor;
    const RED = { nodes: { createNode(n) { n.handlers = {}; n.on = (e, f) => { n.handlers[e] = f; }; n.status = () => {}; n.error = (m) => { throw new Error(m); }; },
                           registerType(_t, c) { Ctor = c; } } };
    registerClassNode(RED, 'benchmesh-psu', 'PSU', { deviceId: 'psu-1', operation: 'query_voltage' });
    const node = {};
    Ctor.call(node, { apiBase: base, deviceId: 'tenma', channel: '1', operation: 'query_voltage' });
    const out = await new Promise((resolve, reject) => node.handlers.input({}, resolve, (e) => e && reject(e)));
    assert.strictEqual(out.value, 4.94);
    await new Promise((resolve, reject) => node.handlers.input({ operation: 'set_voltage', value: '5' }, resolve, (e) => e && reject(e)));
    server.close();
    assert.deepStrictEqual(seen, ['GET /instruments/PSU/tenma/1/voltage', 'POST /instruments/PSU/tenma/1/voltage/5']);
});
