import { useTranslation } from "react-i18next";
// Initialize the shared instance even when a component is rendered alone
// (tests, storybook-like harnesses) rather than through main.tsx.
import "./index";
import type { MessageKey } from "./generated/messageKeys";

export type MessageOptions = Record<string, unknown>;

/**
 * Translation function for components. It re-renders on a language change and
 * only accepts keys generated from en.json.
 */
export function useMessages(): (key: MessageKey, options?: MessageOptions) => string {
  const { t } = useTranslation();
  return (key, options) => (t as unknown as (key: string, options?: MessageOptions) => string)(key, options);
}
