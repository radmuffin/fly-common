/**
 * fly-device-sync.js - Anonymous device token management and API client
 * Handles persistent anonymous user identity, token synchronization, and fetch wrapping.
 */

export class FlyClient {
  /**
   * @param {object} [options]
   * @param {string} [options.storageKey='fly_device_token'] - localStorage key for the device token.
   * @param {string} [options.baseUrl=''] - Base URL prefix for relative API paths.
   */
  constructor(options = {}) {
    this.storageKey = options.storageKey || 'fly_device_token';
    this.baseUrl = (options.baseUrl || '').replace(/\/+$/, '');
    this.token = this.getOrCreateToken();
  }

  /**
   * Load an existing device token from localStorage or generate and persist a new one.
   * Safe to call in private-browsing mode where localStorage may throw.
   * @returns {string}
   */
  getOrCreateToken() {
    let token = null;
    try {
      token = localStorage.getItem(this.storageKey);
    } catch (_) {
      // localStorage unavailable (private browsing / blocked storage)
    }
    if (!token || token.trim() === '') {
      token = (typeof crypto !== 'undefined' && crypto.randomUUID)
        ? crypto.randomUUID().replace(/-/g, '')
        : (Math.random().toString(36).substring(2) + Date.now().toString(36));
      try {
        localStorage.setItem(this.storageKey, token);
      } catch (_) {
        // ignore write failure in restricted environments
      }
    }
    return token;
  }

  /**
   * Overwrite the active device token and persist it.
   * @param {string} newToken
   */
  setToken(newToken) {
    if (newToken && typeof newToken === 'string') {
      this.token = newToken.trim();
      try {
        localStorage.setItem(this.storageKey, this.token);
      } catch (_) {
        // ignore write failure in restricted environments
      }
    }
  }

  /**
   * Send an authenticated fetch request.
   * Automatically attaches the `x-user-token` header and serialises object bodies.
   * @param {string} path - Absolute URL or path relative to `baseUrl`.
   * @param {RequestInit & {body?: object|BodyInit}} [options={}]
   * @returns {Promise<any>} Parsed JSON response.
   * @throws {Error} On non-2xx responses or unsafe URL schemes.
   */
  async request(path, options = {}) {
    const url = path.startsWith('http://') || path.startsWith('https://')
      ? path
      : `${this.baseUrl}${path.startsWith('/') ? path : `/${path}`}`;

    // Reject non-http(s) schemes to prevent data:, javascript: injection
    if (!/^https?:\/\//i.test(url)) {
      throw new Error(`FlyClient: unsafe URL scheme rejected: ${url}`);
    }

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
    } catch (_e) { /* non-JSON response body */ }

    if (!response.ok) {
      const errorMsg = (json && (json.error || json.message)) || `HTTP ${response.status}`;
      throw new Error(errorMsg);
    }

    return json;
  }

  /**
   * Perform a GET request.
   * @param {string} path
   * @param {RequestInit} [options={}]
   */
  get(path, options = {}) {
    return this.request(path, Object.assign({}, options, { method: 'GET' }));
  }

  /**
   * Perform a POST request.
   * @param {string} path
   * @param {object|BodyInit} body
   * @param {RequestInit} [options={}]
   */
  post(path, body, options = {}) {
    return this.request(path, Object.assign({}, options, { method: 'POST', body }));
  }

  /**
   * Perform a PUT request.
   * @param {string} path
   * @param {object|BodyInit} body
   * @param {RequestInit} [options={}]
   */
  put(path, body, options = {}) {
    return this.request(path, Object.assign({}, options, { method: 'PUT', body }));
  }

  /**
   * Perform a DELETE request.
   * @param {string} path
   * @param {RequestInit} [options={}]
   */
  delete(path, options = {}) {
    return this.request(path, Object.assign({}, options, { method: 'DELETE' }));
  }
}
