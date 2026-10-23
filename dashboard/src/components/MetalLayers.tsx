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

/** Shader tint per theme, after Mirror's: warm on stone, white at night. */
const TINT = { light: '#ffb27a', dark: '#ffffff' } as const

/**
 * Mirror's liquid-metal button material: a pill filled with moving chrome
 * (Paper Shaders' LiquidMetal), and a face inset 2px over it, so only a 2px
 * rim of chrome shows. Without WebGL the rim is the CSS conic of chrome bands.
 * Fills its positioned parent; `speed` 0 holds the chrome still.
 */
export function MetalLayers({ speed }: { speed: number }) {
  const theme = useTheme()
  const [shader] = useState(hasWebGL)
  return (
    <>
      <span
        aria-hidden
        className="pointer-events-none absolute inset-0 overflow-hidden rounded-full shadow-[0_0_0_1px_rgb(0_0_0/0.18),0_2px_5px_rgb(0_0_0/0.2),0_9px_14px_-4px_rgb(0_0_0/0.18)]"
        style={shader ? undefined : { background: 'conic-gradient(from 200deg, var(--fw-chrome))' }}
      >
        {shader && (
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
        )}
      </span>
      <span
        aria-hidden
        className="pointer-events-none absolute inset-[2px] rounded-full bg-[image:var(--fw-metal-face)] transition-shadow duration-150 group-active:shadow-[inset_0_2px_4px_rgb(0_0_0/0.4),inset_0_1px_2px_rgb(0_0_0/0.28)]"
      />
    </>
  )
}
