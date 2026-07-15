export function actionCapability(capability, onClick) {
  const button = document.createElement('button');
  button.type = 'button';
  button.textContent = capability.id;
  button.disabled = !capability.enabled;
  if (capability.disabled_reason) button.title = capability.disabled_reason;
  if (capability.enabled) button.addEventListener('click', onClick);
  return button;
}
