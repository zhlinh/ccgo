-- Detect CCGO.toml files
vim.filetype.add({
  filename = {
    ["CCGO.toml"] = "toml",
  },
  pattern = {
    [".*CCGO%.toml"] = "toml",
  },
})
