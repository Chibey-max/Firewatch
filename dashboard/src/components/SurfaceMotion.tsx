import { useEffect } from 'react'

/**
 * One pointer listener for every [data-surface] on the page, after Mirror's.
 * It finds the innermost surface under the mouse, marks it data-hot, and
 * writes where the pointer is into --gx/--gy (in %) for the spotlight and the
 * rim glint in index.css. Writes are batched to one per frame.
 *
 * Desktop only, and off under reduced motion: a phone has no hover, and its
 * surfaces stay still.
 */
export function SurfaceMotion() {
  useEffect(() => {
    if (window.matchMedia('(prefers-reduced-motion: reduce)').matches) return
    if (window.matchMedia('(any-hover: none)').matches) return

    let hot: HTMLElement | null = null
    let pending: PointerEvent | null = null
    let raf = 0

    const apply = () => {
      raf = 0
      const event = pending
      if (!event) return
      const target =
        event.target instanceof Element ? event.target.closest<HTMLElement>('[data-surface]') : null
      if (target !== hot) {
        hot?.removeAttribute('data-hot')
        hot = target
        hot?.setAttribute('data-hot', '')
      }
      if (!hot) return
      const rect = hot.getBoundingClientRect()
      if (!rect.width || !rect.height) return
      const x = Math.min(1, Math.max(0, (event.clientX - rect.left) / rect.width))
      const y = Math.min(1, Math.max(0, (event.clientY - rect.top) / rect.height))
      hot.style.setProperty('--gx', `${(x * 100).toFixed(1)}%`)
      hot.style.setProperty('--gy', `${(y * 100).toFixed(1)}%`)
    }

    // Only a real move lights a surface: Chrome sends a synthetic pointermove
    // at the same spot after a scroll, which shouldn't light what slid under.
    let lastX = Number.NaN
    let lastY = Number.NaN
    const onMove = (event: PointerEvent) => {
      if (event.pointerType !== 'mouse') return
      if (event.clientX === lastX && event.clientY === lastY) return
      lastX = event.clientX
      lastY = event.clientY
      pending = event
      if (!raf) raf = requestAnimationFrame(apply)
    }
    const letGo = () => {
      pending = null
      hot?.removeAttribute('data-hot')
      hot = null
    }

    document.addEventListener('pointermove', onMove, { passive: true })
    document.documentElement.addEventListener('pointerleave', letGo)
    window.addEventListener('scroll', letGo, { passive: true, capture: true })
    return () => {
      document.removeEventListener('pointermove', onMove)
      document.documentElement.removeEventListener('pointerleave', letGo)
      window.removeEventListener('scroll', letGo, { capture: true })
      cancelAnimationFrame(raf)
      letGo()
    }
  }, [])

  return null
}
