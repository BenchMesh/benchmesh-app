// Shared implementation of the per-class nodes (PSU, DMM, ELL).
const { buildRequest, numericValue } = require('./api');

function registerClassNode(RED, type, klass, defaults) {
    function BenchMeshClassNode(config) {
        RED.nodes.createNode(this, config);
        const node = this;
        node.deviceId = config.deviceId || defaults.deviceId;
        node.channel = config.channel || '1';
        node.operation = config.operation || defaults.operation;
        node.value = config.value || defaults.value || '';
        node.apiBase = config.apiBase || 'http://localhost:57666';

        node.on('input', async function(msg, send, done) {
            send = send || function() { node.send.apply(node, arguments); };
            const operation = msg.operation || node.operation;
            let request;
            try {
                const value = msg.value !== undefined ? msg.value
                    : (defaults.valueFromPayload && msg.payload !== undefined ? msg.payload : node.value);
                request = buildRequest(node.apiBase, klass, msg.deviceId || node.deviceId,
                    msg.channel || node.channel, operation, value);
            } catch (e) {
                node.status({ fill: 'red', shape: 'ring', text: e.message });
                return done ? done(e) : node.error(e.message, msg);
            }
            node.status({ fill: 'blue', shape: 'dot', text: operation });
            try {
                const res = await fetch(request.url, { method: request.method });
                const text = await res.text();
                let body = null;
                try { body = text ? JSON.parse(text) : null; } catch (_) { body = text; }
                if (!res.ok) {
                    const detail = (body && body.detail) || `HTTP ${res.status}`;
                    node.status({ fill: 'red', shape: 'ring', text: String(detail).slice(0, 40) });
                    const err = new Error(`${request.method} ${request.url}: ${detail}`);
                    return done ? done(err) : node.error(err.message, msg);
                }
                msg.payload = body === null ? { ok: true } : body;
                if (body && body.value !== undefined) {
                    msg.value = numericValue(body.value);
                    node.status({ fill: 'green', shape: 'dot',
                        text: Number.isNaN(msg.value) ? String(body.value) : msg.value.toFixed(3) });
                } else {
                    node.status({ fill: 'green', shape: 'dot', text: 'ok' });
                }
                send(msg);
                if (done) done();
            } catch (e) {
                node.status({ fill: 'red', shape: 'ring', text: 'error' });
                if (done) done(e); else node.error('Request failed: ' + e.message, msg);
            }
        });
    }
    RED.nodes.registerType(type, BenchMeshClassNode);
}

module.exports = { registerClassNode };
