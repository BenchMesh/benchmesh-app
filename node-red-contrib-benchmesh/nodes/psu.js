const { registerClassNode } = require('./shared/classNode');

module.exports = function(RED) {
    registerClassNode(RED, 'benchmesh-psu', 'PSU', { deviceId: 'psu-1', operation: 'query_voltage' });
};
