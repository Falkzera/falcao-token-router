<script lang="ts">
  // Quantas sessões do Claude Code estão vivas no grupo, e se alguma trabalha
  // ou espera o usuário (≙ SessionsBadge): ponto cheio = há trabalho em curso;
  // vazio = todas ociosas. É o que decide se trocar a conta agora se faz
  // sentir — e é onde o modo de falha silencioso aparece (sessão contada do
  // lado errado).
  import { t } from "../lib/i18n";
  import type { SessionsView } from "../lib/types";

  let { sessions }: { sessions: SessionsView } = $props();

  const engaged = $derived(sessions.engaged > 0);
  const help = $derived(
    engaged
      ? t("groups.sessions.help.active.format", sessions.count, sessions.engaged)
      : t("groups.sessions.help.idle.format", sessions.count, sessions.engaged),
  );
</script>

{#if sessions.count > 0}
  <span class="badge" class:engaged title={help}>
    <span class="dot"></span>{sessions.count}
  </span>
{/if}

<style>
  .badge {
    display: inline-flex;
    align-items: center;
    gap: 3px;
    font-size: var(--font-small);
    font-variant-numeric: tabular-nums;
    color: var(--text-tertiary);
  }
  .dot {
    width: 5px;
    height: 5px;
    border-radius: 50%;
    border: 1px solid currentColor;
  }
  .engaged {
    color: var(--calm);
  }
  .engaged .dot {
    background: currentColor;
  }
</style>
