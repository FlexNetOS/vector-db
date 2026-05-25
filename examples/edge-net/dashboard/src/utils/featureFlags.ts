/**
 * Feature flags
 *
 * Centralizes Vite env-driven toggles for surfaces that are not yet
 * production-ready. Defaults are intentionally conservative: any flag that is
 * unset, empty, or anything other than the string "true" resolves to `false`.
 *
 * See `.env.example` for the canonical list and per-flag rationale.
 */

const isTrue = (value: string | boolean | undefined): boolean => {
  if (typeof value === 'boolean') return value;
  if (typeof value !== 'string') return false;
  return value.trim().toLowerCase() === 'true';
};

const env = import.meta.env as Record<string, string | boolean | undefined>;

export const featureFlags = {
  /** "Restore from encrypted backup" flow in the Identity panel. */
  piKeyRestore: isTrue(env.VITE_ENABLE_PIKEY_RESTORE),
} as const;

export type FeatureFlag = keyof typeof featureFlags;
