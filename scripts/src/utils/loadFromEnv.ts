export function loadFromEnv(key: string, allowNullValue = false): string {
  const value = process.env[key];
  if (!value && !allowNullValue) {
    throw new Error(`${key} not found in env`);
  }
  return value || "";
}
