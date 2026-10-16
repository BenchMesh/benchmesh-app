const { registerClassNode } = require('./shared/classNode');

module.exports = function(RED) {
    registerClassNode(RED, 'benchmesh-dmm', 'DMM', { deviceId: 'dmm-1', operation: 'query_measurement' });
};
