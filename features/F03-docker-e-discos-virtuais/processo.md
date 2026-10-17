# F03 — Coletor Docker e discos virtuais

**Fase:** MVP · **Depende de:** F01, F02

## Objetivo

Encontrar o lixo do Docker (imagens, containers, volumes, cache de build, redes) e o **espaço fantasma** dos discos virtuais do Docker Desktop e das distros WSL, e oferecer a compactação desses discos.

## Por que importa

É o maior item na máquina do usuário (33 GB em discos virtuais). Depois de um prune, o disco virtual **não devolve** o espaço ao Windows sem compactação.

## Fluxo do processo

1. Detectar se o Docker está instalado e se o serviço está rodando.
2. Se estiver rodando: listar imagens, containers, volumes, cache de build e redes, com tamanho e uso.
3. Associar recursos a projetos pelos rótulos do Compose (pasta de trabalho do projeto).
4. Independentemente do Docker estar rodando: medir os discos virtuais (tamanho do arquivo × uso interno, quando possível).
5. Gerar os itens com risco e motivo.
6. Na limpeza: remover os recursos selecionados e, se o usuário pedir, compactar os discos virtuais.

## Regras-chave

- Recurso em uso por container rodando é ⛔ (I5).
- Volumes são sempre 🔴 e vão para a quarentena (I9).
- Compactar exige parar o Docker/WSL: nunca é automático e sempre lista o que será interrompido.
- Docker parado não é erro: o coletor mostra o estado e oferece medir só os discos.

## Ordem das tarefas

T-03.01 → T-03.02 → T-03.03 → T-03.04 → T-03.05 → T-03.06 → T-03.07

## Definição de pronto

Na máquina do usuário, o app mostra quanto do `docker_data.vhdx` é espaço fantasma, permite limpar imagens e cache de build e compacta o disco, devolvendo o espaço ao Windows.
