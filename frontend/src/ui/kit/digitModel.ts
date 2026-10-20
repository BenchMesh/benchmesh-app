/**
 * Arithmetic of the digit-wise setpoint editor: a value shown as `intDigits` + `decimals`
 * fixed digits, where each digit can be stepped or typed like the knob of an instrument.
 * Results are rounded to the shown decimals and clamped to [min, max].
 */
export interface DigitSpec {
  min: number
  max: number
  intDigits: number
  decimals: number
}

export function digitModel({ min, max, intDigits, decimals }: DigitSpec) {
  const total = intDigits + decimals
  const scale = 10 ** decimals
  const round = (v: number) => Math.round(v * scale) / scale
  const clamp = (v: number) => Math.min(max, Math.max(min, round(v)))
  const digits = (v: number) => {
    const s = Math.round(Math.abs(v) * scale).toString().padStart(total, '0')
    return s.slice(-total).split('')
  }
  return {
    total,
    intDigits,
    decimals,
    signed: min < 0,
    clamp,
    digits,
    /** Value with digit `i` (0 = most significant) stepped by `dir`. */
    bump: (v: number, i: number, dir: number) => clamp(v + dir * 10 ** (intDigits - 1 - i)),
    /** Value with digit `i` replaced by `d`, keeping the sign. */
    type: (v: number, i: number, d: string) => {
      const ds = digits(v)
      ds[i] = d
      return clamp((v < 0 ? -1 : 1) * Number(ds.join('')) / scale)
    },
    /** Sign flipped (only for ranges that allow negative values). */
    negate: (v: number) => (min < 0 ? clamp(-v) : v),
  }
}

export type DigitModel = ReturnType<typeof digitModel>
