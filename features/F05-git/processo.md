# F05 — Coletor Git

**Fase:** MVP · **Depende de:** F01, F02

## Objetivo

Encontrar branches locais que já foram mergeadas (inclusive por squash e rebase), worktrees perdidas ou concluídas e stashes antigos, sem nunca perder trabalho.

## Por que importa

Agentes de IA criam uma branch e muitas vezes uma worktree por tarefa. O ganho principal é de **organização**; as worktrees também trazem ganho de **espaço**, porque cada uma tem suas próprias dependências e builds.

## Fluxo do processo

1. Inventariar os repositórios dentro das pastas raiz: branch padrão, remotos, estado da cópia de trabalho.
2. Para cada branch local, coletar sinais de merge locais (ancestralidade, upstream apagado, conteúdo equivalente).
3. Quando houver remoto no GitHub e autenticação disponível, consultar o estado dos PRs.
4. Consolidar o veredito de cada branch e o risco.
5. Analisar worktrees (pasta inexistente, branch mergeada, alterações pendentes, arquivos ignorados valiosos).
6. Analisar stashes por idade.
7. Na limpeza: tudo vai para a quarentena (I9), de forma restaurável.

## Regras-chave

- Nunca candidatas: branch atual, branch padrão, branches protegidas, branches com commits nunca enviados que não estejam contidos na branch padrão (I2).
- Uma branch só é "mergeada via PR" se o commit local for o mesmo (ou ancestral) do último commit do PR mergeado.
- Sem rede ou sem autenticação, o coletor funciona só com sinais locais e indica "verificação limitada".

## Ordem das tarefas

T-05.01 → T-05.02 → T-05.03 → T-05.04 → T-05.05 → T-05.06

## Definição de pronto

Num repositório de teste com merges normais, squash, rebase, branches abandonadas e worktrees, o app classifica cada caso corretamente e remove e restaura via quarentena sem perda.
