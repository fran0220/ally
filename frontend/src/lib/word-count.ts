/**
 * 按 Word 统计规则计算字数：
 * 1. 中文字符每个计 1
 * 2. 英文/数字连续片段计 1
 */
export function countWords(text: string): number {
  if (!text) {
    return 0;
  }

  let englishWordCount = 0;
  const textWithoutEnglish = text.replace(/[a-zA-Z0-9]+/g, () => {
    englishWordCount += 1;
    return '';
  });

  const chineseMatches = textWithoutEnglish.match(/[\u4e00-\u9fa5\u3400-\u4dbf\u20000-\u2a6df]/g);
  const chineseCount = chineseMatches ? chineseMatches.length : 0;

  return englishWordCount + chineseCount;
}

export function countCharacters(text: string): number {
  return text.length;
}

export function countCharactersNoSpaces(text: string): number {
  return text.replace(/\s/g, '').length;
}
