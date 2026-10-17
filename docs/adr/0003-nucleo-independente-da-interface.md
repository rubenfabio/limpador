# ADR-0003 — Núcleo como biblioteca independente da interface

- **Status:** Aceita
- **Data:** 2026-10-16
- **Decisores:** Dono do produto
- **Tarefas relacionadas:** T-00.01, F01, T-12.01, T-12.02

## Contexto

As mesmas regras (análise, risco, limpeza, quarentena, histórico) serão usadas pelo app desktop, por uma CLI e por um servidor MCP para agentes (Fase 2). Se as regras ficarem misturadas com a interface, cada canal reimplementa a lógica, e as regras de segurança divergem.

## Decisão

Toda regra de negócio fica numa **biblioteca de núcleo** em Rust, sem nenhuma dependência de Tauri ou da interface. O app desktop, a CLI e o servidor MCP são **cascas finas** que chamam o núcleo.

```
           ┌──────────── núcleo (biblioteca) ────────────┐
           │ domínio · motor · coletores · quarentena ·  │
           │ histórico · preferências                    │
           └───────▲──────────────▲──────────────▲───────┘
                   │              │              │
            app desktop         CLI         servidor MCP
            (Tauri + UI)     (Fase 2)        (Fase 2)
```

## Alternativas consideradas

| Alternativa | Prós | Contras |
|---|---|---|
| **Núcleo separado** | Uma única fonte de regras; testável sem interface; reuso na Fase 2 | Exige disciplina de fronteira desde o início |
| Lógica nos comandos do Tauri | Mais rápido no começo | Impossível reutilizar; testes dependem do app |

## Consequências

**Positivas**
- Testes de unidade e integração rodam sem compilar a interface.
- As regras de segurança (I1–I10) existem num só lugar.

**Negativas / custos aceitos**
- Uma camada de tradução entre os tipos do núcleo e os da interface.

**O que passa a ser obrigatório**
- O núcleo não importa nada de Tauri nem da interface.
- As cascas não contêm regra de negócio: validação de risco, destino e permissões acontece no núcleo.
