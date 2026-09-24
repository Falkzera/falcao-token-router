<script lang="ts">
  // O interruptor do Windows 11 (liga/desliga), com o rótulo clicável.
  // CONTROLADO: o clique só pede; a tela mostra o que o backend confirmar. Um
  // "Abrir no login" que o Windows recusou volta a `false` — e, sem isto, o
  // valor não mudaria, o Svelte não tocaria no DOM e o interruptor ficaria
  // ligado sobre um registro que não existe.
  let {
    checked,
    label,
    onChange,
    disabled = false,
  }: { checked: boolean; label: string; onChange: (on: boolean) => void; disabled?: boolean } =
    $props();
</script>

<label class="switch" class:disabled>
  <input
    type="checkbox"
    role="switch"
    {checked}
    {disabled}
    onchange={(event) => {
      const on = event.currentTarget.checked;
      event.currentTarget.checked = checked;
      onChange(on);
    }}
  />
  <span class="track" aria-hidden="true"><span class="thumb"></span></span>
  <span class="label">{label}</span>
</label>

<style>
  .switch {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    font-size: var(--font-caption);
    cursor: pointer;
  }
  .switch.disabled {
    opacity: 0.5;
    cursor: default;
  }
  input {
    position: absolute;
    opacity: 0;
    pointer-events: none;
  }
  .track {
    position: relative;
    width: 34px;
    height: 18px;
    border: 1px solid var(--text-secondary);
    border-radius: 9px;
    transition: background 120ms;
  }
  .thumb {
    position: absolute;
    top: 3px;
    left: 3px;
    width: 10px;
    height: 10px;
    border-radius: 50%;
    background: var(--text-secondary);
    transition: transform 120ms;
  }
  input:checked + .track {
    border-color: var(--accent);
    background: var(--accent);
  }
  input:checked + .track .thumb {
    transform: translateX(16px);
    background: var(--accent-text);
  }
  input:focus-visible + .track {
    outline: 2px solid var(--focus);
    outline-offset: 2px;
  }
</style>
