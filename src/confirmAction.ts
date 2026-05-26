export async function runConfirmed(
  confirm: (message: string) => boolean,
  message: string,
  action: () => void | Promise<void>,
): Promise<boolean> {
  if (!confirm(message)) {
    return false;
  }

  await action();
  return true;
}
