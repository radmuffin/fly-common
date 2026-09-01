/**
 * fly-ui.js - Minimal UI components (Toasts, Modals, Theme) for Fly.io SPAs
 * Zero dependencies, pure ES6 module.
 */

export class FlyToast {
  static container = null;

  /**
   * Ensure the toast container element exists in the DOM.
   */
  static init() {
    if (!this.container) {
      this.container = document.createElement('div');
      this.container.className = 'fly-toast-container';
      document.body.appendChild(this.container);
    }
  }

  /**
   * Display a toast notification.
   * @param {string} message - Text to display (HTML-escaped automatically).
   * @param {'info'|'success'|'error'} [type='info'] - Visual variant.
   * @param {number} [duration=3500] - Auto-dismiss delay in milliseconds.
   */
  static show(message, type = 'info', duration = 3500) {
    this.init();
    const toast = document.createElement('div');
    toast.className = `fly-toast fly-toast-${type}`;
    toast.setAttribute('role', 'alert');
    toast.innerHTML = `<span>${FlyToast.escape(message)}</span>`;

    this.container.appendChild(toast);

    // Trigger animation
    requestAnimationFrame(() => {
      toast.classList.add('fly-toast-visible');
    });

    setTimeout(() => {
      toast.classList.remove('fly-toast-visible');
      setTimeout(() => toast.remove(), 300);
    }, duration);
  }

  /** @param {string} msg @param {number} [dur] */
  static success(msg, dur) { this.show(msg, 'success', dur); }
  /** @param {string} msg @param {number} [dur] */
  static error(msg, dur) { this.show(msg, 'error', dur); }
  /** @param {string} msg @param {number} [dur] */
  static info(msg, dur) { this.show(msg, 'info', dur); }

  /**
   * HTML-escape a string to prevent XSS when inserting into the DOM.
   * @param {string} str
   * @returns {string}
   */
  static escape(str) {
    if (!str) return '';
    return String(str).replace(/[&<>"']/g, m => ({
      '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;'
    }[m]));
  }
}

export class FlyTheme {
  static STORAGE_KEY = 'fly_theme_mode';

  /**
   * Read the stored theme preference and apply it.
   * Safe to call in private-browsing mode where localStorage may throw.
   */
  static init() {
    let saved = 'system';
    try {
      saved = localStorage.getItem(this.STORAGE_KEY) || 'system';
    } catch (_) {
      // localStorage unavailable (private browsing / blocked storage)
    }
    this.apply(saved);
  }

  /**
   * Apply a theme mode and persist the preference.
   * @param {'light'|'dark'|'system'} mode
   */
  static apply(mode) {
    try {
      localStorage.setItem(this.STORAGE_KEY, mode);
    } catch (_) {
      // ignore write failure in restricted environments
    }
    if (mode === 'dark' || (mode === 'system' && window.matchMedia('(prefers-color-scheme: dark)').matches)) {
      document.documentElement.setAttribute('data-theme', 'dark');
    } else {
      document.documentElement.setAttribute('data-theme', 'light');
    }
  }

  /**
   * Toggle between light and dark themes.
   * @returns {'light'|'dark'} The newly applied theme.
   */
  static toggle() {
    const current = document.documentElement.getAttribute('data-theme');
    const next = current === 'dark' ? 'light' : 'dark';
    this.apply(next);
    return next;
  }
}

// Auto-initialize theme on load
if (typeof document !== 'undefined') {
  FlyTheme.init();
}
