# ADR-0004 — Coletores plugáveis com contrato único e dependências injetáveis

- **Status:** Aceita
- **Data:** 2026-10-16
- **Decisores:** Dono do produto
- **Tarefas relacionadas:** T-01.04, F03–F07

## Contexto

Cada ecossistema (Docker, Git, Node, Python, ferramentas de IA...) tem regras próprias e muda em ritmo próprio. Novos ecossistemas vão surgir. Os coletores precisam ser testados sem Docker real, sem a pasta real do usuário e sem relógio real.

## Decisão

- Todo coletor implementa o mesmo contrato: **detectar · analisar · revalidar · limpar · restaurar** (T-01.04).
- Os coletores são **registrados** no motor; adicionar um não altera o motor.
- Coletores não acessam o sistema diretamente: recebem **abstrações injetadas** para sistema de arquivos, execução de comandos externos (`git`, `docker`, `wsl`), API do GitHub e relógio.
- No registro, um coletor que declara tipos "conteúdo do usuário" sem restauração é rejeitado (I9).

## Alternativas consideradas

| Alternativa | Prós | Contras |
|---|---|---|
| **Contrato único + injeção** | Testável com dublês; extensível; o motor trata todos igual | Mais interfaces para manter |
| Um módulo monolítico por categoria | Menos abstração | Difícil de testar e estender |
| Plugins carregados em tempo de execução | Extensão por terceiros | Complexidade e risco de segurança desnecessários agora |

## Consequências

**Positivas**
- A máquina falsa (ADR-0012) se encaixa naturalmente.
- Novos ecossistemas entram com uma implementação e cenários de teste.

**O que passa a ser obrigatório**
- Nenhum coletor chama processo, disco ou rede sem passar pelas abstrações.
- Plugins de terceiros ficam fora do escopo até um ADR novo.
