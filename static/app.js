async function refreshStatus() {
  try {
    const res = await fetch('/api/status', { headers: { accept: 'application/json' } });
    const status = await res.json();
    document.documentElement.dataset.arcadia = status.arcadia?.mode || 'unknown';
    document.documentElement.dataset.product = status.product || 'HomeConsole';
  } catch (err) {
    console.warn('Arcadia status unavailable', err);
  }
}
refreshStatus();
setInterval(refreshStatus, 30000);
