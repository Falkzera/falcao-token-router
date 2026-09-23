// O relógio da tela: idades de amostra e tempo até o reset são contados a
// partir daqui, e ele anda a cada 30 s (o mesmo passo com que o backend relê o
// quadro) — um "3m" parado na tela aberta seria um número que envelhece calado.

export const clock = $state({ now: Date.now() });

setInterval(() => {
  clock.now = Date.now();
}, 30_000);
