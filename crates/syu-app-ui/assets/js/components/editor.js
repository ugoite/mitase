export function structuredEditor(value, onSubmit) {
  const form = document.createElement('form');
  const input = document.createElement('textarea');
  input.value = value;
  const submit = document.createElement('button');
  submit.type = 'submit';
  submit.textContent = 'Apply';
  form.append(input, submit);
  form.addEventListener('submit', event => { event.preventDefault(); onSubmit(input.value); });
  return form;
}
