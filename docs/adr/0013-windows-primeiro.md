# ADR-0013 — Windows primeiro, com diferenças de plataforma isoladas

- **Status:** Aceita
- **Data:** 2026-10-16
- **Decisores:** Dono do produto
- **Tarefas relacionadas:** todas, em especial F03, F07, T-03.07

## Contexto

O dono do produto usa Windows com WSL e Docker Desktop. Parte do maior ganho (discos virtuais `.vhdx`, ~33 GB na máquina dele) só existe no Windows. O plano é portar para Mac e Linux depois.

## Decisão

- O MVP mira **apenas Windows**.
- Tudo o que depende de plataforma (locais padrão, discos virtuais, elevação de privilégio, bandeja, detecção de arquivo em uso, lixeira) fica atrás de abstrações de plataforma no núcleo, com implementação Windows.
- Os locais por plataforma vêm do catálogo (ADR-0005).

## Alternativas consideradas

| Alternativa | Prós | Contras |
|---|---|---|
| **Windows primeiro, isolado** | Foco na dor real; porte futuro previsto | Mac/Linux esperam |
| Três plataformas desde o início | Alcance | Triplica coletores e testes no MVP |
| Só Windows, sem abstração | Mais rápido no início | Porte vira reescrita |

## Consequências

- Os testes cobrem as particularidades do Windows (caminhos longos, junções, arquivos travados, maiúsculas e minúsculas).
- O CI roda em Windows (ADR-0011).
