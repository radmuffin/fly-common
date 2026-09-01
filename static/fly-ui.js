/**
 * fly-ui.js - Minimal UI components (Toasts, Modals, Theme) for Fly.io SPAs
 * Zero dependencies, pure ES6 module.
 */

export class FlyToast {
  static container = null;

  static init() {
    if (!this.container) {
      this.container = document.createElement('div');
      this.container.className = 'fly-toast-container';
      document.body.appendChild(this.container);
    }
  }

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

  static success(msg, dur) { this.show(msg, 'success', dur); }
  static error(msg, dur) { this.show(msg, 'error', dur); }
  static info(msg, dur) { this.show(msg, 'info', dur); }

  static escape(str) {
    if (!str) return '';
    return String(str).replace(/[&<>"']/g, m => ({
      '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;'
    }[m]));
  }
}

export class FlyTheme {
  static STORAGE_KEY = 'fly_theme_mode';

  static init() {
    const saved = localStorage.getItem(this.STORAGE_KEY) || 'system';
    this.apply(saved);
  }

  static apply(mode) {
    localStorage.setItem(this.STORAGE_KEY, mode);
    if (mode === 'dark' || (mode === 'system' && window.matchMedia('(prefers-color-scheme: dark)').matches)) {
      document.documentElement.setAttribute('data-theme', 'dark');
    } else {
      document.documentElement.setAttribute('data-theme', 'light');
    }
  }

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
