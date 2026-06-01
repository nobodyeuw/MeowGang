export function reloadTenorEmbeds() {
  if (typeof window === 'undefined') {
    return;
  }

  window.setTimeout(() => {
    document.querySelector('script[data-tenor-reload="true"]')?.remove();
    const script = document.createElement('script');
    script.src = 'https://tenor.com/embed.js';
    script.async = true;
    script.setAttribute('data-tenor-reload', 'true');
    document.body.appendChild(script);
  }, 0);
}
