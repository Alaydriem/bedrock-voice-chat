import type { Severity } from "$radial/core/controllers/Diagnostics";
import type { IconName } from "$radial/core/icons/Icons";
import type FaultAction from "./FaultAction";

/** One terminal state: what broke, how it reads, and where it can send someone. */
export default interface FaultDefinition {
  code: string;
  title: string;
  message: string;
  /** The glyph at the centre of the break. */
  icon: IconName;
  /**
   * bad · something is broken. warn · a person has to act before this can work, and
   * nothing here is anyone's mistake. ok · not a failure at all.
   */
  severity: Severity;
  /** What this is about: the caption's label, and the eyebrow over the title. */
  category: string;
  /**
   * Replaces the eyebrow with a severity chip. Only the screens that are not reporting a
   * break have one, because leading with "nothing is wrong" is only worth doing when it is
   * true.
   */
  chip?: string;
  /** The state in two or three words. Joined to the code in the caption. */
  caption: string;
  /** Right of the top bar. */
  label: string;
  /** Left of the footbar: the usual cause, or a reassurance. */
  hint: string;
  primaryAction: FaultAction;
  secondaryAction?: FaultAction;
}
