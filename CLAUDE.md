@AGENTS.md

## Notas específicas para o Claude Code

- As instruções acima (AGENTS.md) são a fonte da verdade. Não duplique regras aqui: altere o AGENTS.md.
- Antes de implementar uma tarefa, leia o `processo.md` da feature e a tarefa inteira. Se um critério de aceite for ambíguo, pergunte ao usuário com opções em vez de supor.
- O usuário prefere decidir em bate e volta: ao surgir uma decisão de produto, apresente opções com uma recomendada.
- Para documentação de bibliotecas (Tauri, crates Rust, React), consulte a documentação atual via ctx7 antes de escrever código dependente de API.
- Ironia a evitar: se usar worktrees para trabalhar, remova-as ao terminar.
