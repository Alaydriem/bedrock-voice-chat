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
    let reason = await FormShow.showOrClosed(form);
    for (
      let attempt = 0;
      reason === DataDrivenScreenClosedReason.UserBusy &&
      attempt < USER_BUSY_RETRIES;
      attempt++
    ) {
      await FormShow.wait(USER_BUSY_RETRY_TICKS);
      reason = await FormShow.showOrClosed(form);
    }
    return reason;
  }

  // show() REJECTS (PlayerLeftError / ServerShutdownError) when the viewer
  // disconnects with the form open. That is a normal way for a session to end
  // — fold it into ClientClosed so callers run their teardown instead of
  // leaking an unhandled rejection (and their subscriptions with it).
  private static async showOrClosed(
    form: CustomForm,
  ): Promise<DataDrivenScreenClosedReason> {
    try {
      return await form.show();
    } catch {
      return DataDrivenScreenClosedReason.ClientClosed;
    }
  }

  private static wait(ticks: number): Promise<void> {
    return new Promise((resolve) => system.runTimeout(resolve, ticks));
  }
}
