export function env(key: string, allowNullValue?: boolean): string {
  const value = process.env[key];
  if (!value && !allowNullValue) {
    throw new Error(`${key} not found in env`);
  }
  return value || "";
}
