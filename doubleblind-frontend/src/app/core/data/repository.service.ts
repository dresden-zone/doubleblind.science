import { Injectable } from '@angular/core';
import {map, Observable, of} from "rxjs";
import {Repository} from "./repository.domain";
import {HttpClient} from "@angular/common/http";
import {API_URL} from "./api.domain";

@Injectable({
  providedIn: 'root'
})
export class RepositoryService {

  constructor(
    private readonly http: HttpClient
  ) { }


  public getUserRepos() : Observable<Repository[]> {
    return this.http.get<Repository[]>(`https://api.${API_URL}/v1/github/repos`,{
      withCredentials: true
    })
    /*return of([
      {
        id: BigInt(1231),
        name: "Test1",
        full_name: "xxx/Test1",
        deployed: true,
        domain: "domain123",
        branch: "master",
      },
      {
        id: BigInt(11231),
        name: "Test2",
        full_name: "yyy/Test2",
        deployed: false,
        domain: undefined,
        branch: undefined,
      }
    ])*/
  }
  public deployRepo(domain: string, branch: string, github_id: bigint): Observable<void> {
    console.log("creating project with " + domain + " and repo " + github_id);
    return this.http.post(`https://api.${API_URL}/v1/github/deploy`, {
      domain: domain,
      branch: branch,
      github_id: github_id
    }, {
      withCredentials: true
    }).pipe(map(() => void 0));
  }
}
