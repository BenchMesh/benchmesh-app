// BenchMesh API helpers shared by the nodes (no dependencies).
//
// The API takes partial operation names: GET .../{ch}/voltage calls the device's
// voltage query, POST .../{ch}/voltage/{value} sets it. The node editors store
// descriptive operation ids ("query_voltage", "set_output"); this module maps them
// to the verb + operation name of the instrument class contract.

// Editor labels whose class operation has a different name.
const CLASS_ALIASES = {
    ELL: { current: 'curr', voltage: 'volt', power: 'pow', resistance: 'res' },
};

function resolveOperation(klass, operation) {
    const op = String(operation || '').trim();
    let verb = 'GET';
    let name = op;
    if (op.startsWith('set_')) {
        verb = 'POST';
        name = op.slice(4);
    } else if (op.startsWith('query_')) {
        name = op.slice(6);
    }
    const aliases = CLASS_ALIASES[klass] || {};
    return { verb, name: aliases[name] || name };
}

function buildRequest(apiBase, klass, deviceId, channel, operation, value) {
    const { verb, name } = resolveOperation(klass, operation);
    const base = String(apiBase || 'http://localhost:57666').replace(/\/+$/, '');
    let url = `${base}/instruments/${klass}/${encodeURIComponent(deviceId)}/${channel}/${name}`;
    if (verb === 'POST') {
        if (value === undefined || value === null || value === '') {
            throw new Error(`operation ${operation} needs a value`);
        }
        url += `/${encodeURIComponent(String(value))}`;
    }
    return { method: verb, url };
}

// Numeric value of an API result: plain numbers, numeric strings and BenchMesh SI value
// dicts ({si, sci, val}). NaN when not numeric.
function numericValue(value) {
    if (typeof value === 'number') return value;
    if (value && typeof value === 'object') {
        const raw = value.sci !== undefined ? value.sci : value.val;
        return raw === undefined ? NaN : parseFloat(raw);
    }
    return parseFloat(value);
}

module.exports = { resolveOperation, buildRequest, numericValue };
