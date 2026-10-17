// Every visible text of the interface (T-08.07, ADR-0014, ADR-0019).
// Placeholders use {name}. Plural entries pick a form from Intl.PluralRules by {count}.
import type { Dictionary } from "./index";

export const ptBR: Dictionary = {
  "app.name": "Limpador",
  "app.version": "v{version}",
  "common.items": { zero: "nenhum item", one: "{count} item", other: "{count} itens" },

  "onboarding.skip": "Pular por agora",
  "onboarding.start": "Começar",
  "onboarding.next": "Continuar",
  "onboarding.finish": "Concluir e analisar",
  "onboarding.welcome.title": "Boas-vindas ao Limpador",
  "onboarding.welcome.what":
    "O Limpador encontra o que o desenvolvimento acumula no seu computador (Docker, dependências, branches, caches) e mostra o que dá para liberar com segurança.",
  "onboarding.welcome.safe": "Analisar nunca apaga nada. Você decide o que limpar.",
  "onboarding.roots.title": "Onde ficam seus projetos?",
  "onboarding.roots.help":
    "Escolha as pastas que o Limpador deve analisar. Nada fora delas é tocado.",
  "onboarding.roots.searching": "Procurando pastas com projetos…",
  "onboarding.roots.none_found": "Nenhuma pasta com projetos foi encontrada. Adicione uma.",
  "onboarding.roots.projects": {
    zero: "nenhum projeto",
    one: "{count} projeto",
    other: "{count} projetos",
  },
  "onboarding.roots.add": "Adicionar outra pasta",
  "onboarding.autonomy.title": "Limpeza automática do que é seguro",
  "onboarding.autonomy.help":
    "Itens seguros são caches que se recriam sozinhos, sem você fazer nada e sem perda. Todo o resto sempre pede sua confirmação.",
  "onboarding.autonomy.option": "Limpar automaticamente os itens seguros",
  "onboarding.background.title": "Trabalhar em segundo plano",
  "onboarding.background.help":
    "O Limpador fica na bandeja, analisa periodicamente e avisa quando vale a pena limpar.",
  "onboarding.background.option": "Iniciar com o Windows",

  "roots.not_found": "Essa pasta não existe.",
  "roots.not_readable": "Não foi possível ler essa pasta.",
  "roots.unavailable": "A pasta {path} não está disponível agora (disco desconectado?).",

  "home.title": "Visão geral",
  "home.no_roots.title": "Nenhuma pasta configurada",
  "home.no_roots.help": "Escolha onde ficam seus projetos para o Limpador começar a analisar.",
  "home.no_roots.action": "Configurar pastas",
  "home.never_analyzed": "Ainda não houve nenhuma análise.",
  "home.analyzing": "Analisando…",
  "home.analyze": "Analisar agora",
  "home.recoverable": "Pode ser liberado",
  "home.recoverable_safe": "Disso, seguro",
  "home.projects": "Projetos encontrados",
  "home.projects_count": {
    zero: "nenhum projeto",
    one: "{count} projeto",
    other: "{count} projetos",
  },
};
