/**
 * Drop every service worker on this origin and the caches they filled, then
 * reload. serve.py shares the dev server's origin and registers the PWA
 * worker, whose precache keeps serving an old engine. The document in
 * localStorage is kept.
 * @param {{ reload?: boolean }} [opts]
 * @returns {Promise<number>} how many workers were removed
 */
export async function clearServiceWorkers({ reload = true } = {}) {
  const regs = 'serviceWorker' in navigator ? await navigator.serviceWorker.getRegistrations() : []
  await Promise.all(regs.map((r) => r.unregister()))
  if ('caches' in window) {
    await Promise.all((await caches.keys()).map((k) => caches.delete(k)))
  }
  if (reload) location.reload()
  return regs.length
}
