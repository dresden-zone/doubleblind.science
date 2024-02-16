export interface Repository {
  id: bigint,
  name: string,
  full_name: string,
  deployed: boolean,
  domain: string | undefined,
  branch: string | undefined
}

