import { ApiSource } from "./types";

interface SourceSelectionSearch {
  nextPage: number;
  nextQuery: string;
  nextSource: ApiSource;
}

export function sourceSelectionSearch(
  query: string,
  source: ApiSource,
): SourceSelectionSearch {
  return {
    nextPage: 1,
    nextQuery: query,
    nextSource: source,
  };
}
