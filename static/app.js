async function refreshStatus() {
  try {
    const res = await fetch('/api/status', { headers: { accept: 'application/json' } });
    const status = await res.json();
    document.documentElement.dataset.arcadia = status.arcadia?.mode || 'scaffold';
    console.log('Arcadia status', status);
  } catch (err) {
    console.warn('Arcadia status unavailable', err);
  }
}

refreshStatus();
