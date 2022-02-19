{
  "parser": "@typescript-eslint/parser",
  "parserOptions": {
    "ecmaVersion": 2020,
    "sourceType": "module"
  },
  "extends": ["plugin:@typescript-eslint/recommended", "plugin:prettier/recommended"],
  "rules": {
    "prefer-const": "warn",
    "no-extra-semi": "off",
    "@typescript-eslint/no-extra-semi": "off",
    "@typescript-eslint/no-inferrable-types": "warn",
    "@typescript-eslint/no-empty-function": "warn",
    "no-only-tests/no-only-tests": "error"
  },
  "plugins": [
    "no-only-tests"
  ]
}