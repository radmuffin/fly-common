/**
 * fly-device-sync.js - Anonymous device token management and API client
 * Handles persistent anonymous user identity, token synchronization, and fetch wrapping.
 */

export class FlyClient {
  constructor(options = {}) {
    this.storageKey = options.storageKey || 'fly_device_token';
    this.baseUrl = (options.baseUrl || '').replace(/\/+$/, '');
    this.token = this.getOrCreateToken();
  }

  getOrCreateToken() {
    let token = localStorage.getItem(this.storageKey);
    if (!token || token.trim() === '') {
      token = (typeof crypto !== 'undefined' && crypto.randomUUID)
        ? crypto.randomUUID().replace(/-/g, '')
        : (Math.random().toString(36).substring(2) + Date.now().toString(36));
      localStorage.setItem(this.storageKey, token);
    }
    return token;
  }

  setToken(newToken) {
    if (newToken && typeof newToken === 'string') {
      this.token = newToken.trim();
      localStorage.setItem(this.storageKey, this.token);
    }
  }

  async request(path, options = {}) {
    const url = path.startsWith('http://') || path.startsWith('https://')
      ? path
      : `${this.baseUrl}${path.startsWith('/') ? path : `/${path}`}`;

    const headers = Object.assign(
      {
        'x-user-token': this.token,
        'Accept': 'application/json',
      },
      options.headers || {}
    );

    if (options.body && typeof options.body === 'object' && !(options.body instanceof FormData)) {
      headers['Content-Type'] = 'application/json';
      options.body = JSON.stringify(options.body);
    }

    const response = await fetch(url, Object.assign({}, options, { headers }));
    let json = null;
    try {
      json = await response.json();
    } catch (_) {}

    if (!response.ok) {
      const errorMsg = (json && (json.error || json.message)) || `HTTP ${response.status}`;
      throw new Error(errorMsg);
    }

    return json;
  }

  get(path, options = {}) {
    return this.request(path, Object.assign({}, options, { method: 'GET' }));
  }

  post(path, body, options = {}) {
    return this.request(path, Object.assign({}, options, { method: 'POST', body }));
  }

  put(path, body, options = {}) {
    return this.request(path, Object.assign({}, options, { method: 'PUT', body }));
  }

  delete(path, options = {}) {
    return this.request(path, Object.assign({}, options, { method: 'DELETE' }));
  }
}
