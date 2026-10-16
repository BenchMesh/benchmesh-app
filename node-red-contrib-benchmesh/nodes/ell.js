const { registerClassNode } = require('./shared/classNode');

module.exports = function(RED) {
    registerClassNode(RED, 'benchmesh-ell', 'ELL',
        { deviceId: 'eol-1', operation: 'set_input', value: 'OFF', valueFromPayload: true });
};
