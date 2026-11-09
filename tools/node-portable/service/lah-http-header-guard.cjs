const http = require('http');
const installed = Symbol.for('lah.httpHeaderGuard');
const blocked = name => typeof name === 'string' && ['x-powered-by', 'server'].includes(name.toLowerCase());
const prototype = http.ServerResponse.prototype;

function filterHeaders(headers, nested = false) {
  if (Array.isArray(headers)) {
    if (nested) return headers.filter(pair => !(Array.isArray(pair) && blocked(pair[0])));
    if (headers.length % 2) return headers;
    const filtered = [];
    for (let i = 0; i < headers.length; i += 2) {
      if (!blocked(headers[i])) filtered.push(headers[i], headers[i + 1]);
    }
    return filtered;
  }
  return headers && typeof headers === 'object'
    ? Object.fromEntries(Object.entries(headers).filter(([name]) => !blocked(name)))
    : headers;
}

if (!prototype[installed]) {
  const setHeader = prototype.setHeader;
  const writeHead = prototype.writeHead;
  prototype.setHeader = function(name, value) {
    return blocked(name) ? this : setHeader.call(this, name, value);
  };
  prototype.writeHead = function(...args) {
    this.removeHeader('X-Powered-By');
    this.removeHeader('Server');
    const index = typeof args[1] === 'string' ? 2 : 1;
    const headers = args[index];
    args[index] = filterHeaders(headers);
    return writeHead.apply(this, args);
  };
  const addTrailers = prototype.addTrailers;
  prototype.addTrailers = function(headers) {
    return addTrailers.call(this, filterHeaders(headers, true));
  };
  if (typeof prototype.writeEarlyHints === 'function') {
    const writeEarlyHints = prototype.writeEarlyHints;
    prototype.writeEarlyHints = function(...args) {
      args[0] = filterHeaders(args[0]);
      return writeEarlyHints.apply(this, args);
    };
  }
  Object.defineProperty(prototype, installed, { value: true });
}
