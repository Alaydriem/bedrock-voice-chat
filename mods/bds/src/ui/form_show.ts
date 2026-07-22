import { system } from '@minecraft/server';
import type { CustomForm } from '@minecraft/server-ui';
import { DataDrivenScreenClosedReason } from '@minecraft/server-ui';

// The screen a form replaces (chat, another form mid-teardown) may still be
// closing when show() first runs; retry a few times before giving up.
const USER_BUSY_RETRIES = 5;
const USER_BUSY_RETRY_TICKS = 10;

/// Shows a DDUI form, retrying through the UserBusy window. Shared by the panel
/// and every volumes page so back-to-back form navigation survives teardown lag.
export class FormShow {
  static async withRetry(form: CustomForm): Promise<DataDrivenScreenClosedReason> {
    let reason = await form.show();
    for (
      let attempt = 0;
      reason === DataDrivenScreenClosedReason.UserBusy &&
      attempt < USER_BUSY_RETRIES;
      attempt++
    ) {
      await FormShow.wait(USER_BUSY_RETRY_TICKS);
      reason = await form.show();
    }
    return reason;
  }

  private static wait(ticks: number): Promise<void> {
    return new Promise((resolve) => system.runTimeout(resolve, ticks));
  }
}
