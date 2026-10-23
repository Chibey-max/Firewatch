import { LiquidMetal } from '@paper-design/shaders-react'
import { useState } from 'react'
import { useTheme } from '../lib/useTheme'

/** Whether this browser can draw the shader. Checked once. */
let webgl: boolean | null = null
function hasWebGL(): boolean {
  if (webgl === null) {
    try {
      webgl = !!document.createElement('canvas').getContext('webgl2')
    } catch {
      webgl = false
    }
  }
  return webgl
}

/** Shader tint per theme: warm bronze on stone, white at night. */
const TINT = { light: '#ffb27a', dark: '#ffffff' } as const

/**
 * Liquid metal as a thin rim round clear glass: Paper Shaders' LiquidMetal
 * fills the pill and .fw-ring-mask cuts it down to a 1.5px ring, so the face
 * stays as transparent as every other pane. Without WebGL the ring is the CSS
 * conic of chrome bands, turning. Fills its positioned parent; `speed` 0 holds
 * the metal still.
 */
export function LiquidRim({ speed }: { speed: number }) {
  const theme = useTheme()
  const [shader] = useState(hasWebGL)
  if (!shader) return <span aria-hidden className="fw-chrome-rim" />
  return (
    <span
      aria-hidden
      className="fw-ring-mask pointer-events-none absolute inset-0 rounded-[inherit]"
    >
      <LiquidMetal
        className="absolute inset-0 h-full w-full"
        speed={speed}
        repetition={4}
        softness={0.5}
        shiftRed={0.3}
        shiftBlue={0.3}
        distortion={0}
        contour={0}
        angle={45}
        scale={8}
        shape="circle"
        offsetX={0.1}
        offsetY={-0.1}
        colorTint={TINT[theme]}
      />
    </span>
  )
}
