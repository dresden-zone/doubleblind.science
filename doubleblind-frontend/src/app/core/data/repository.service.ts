import { Injectable } from '@angular/core';
import {Observable, of} from "rxjs";
import {Project} from "./project.domain";
import {Repository} from "./repository.domain";
import {API_URL} from "./api.domain";
import {HttpClient, HttpParams} from "@angular/common/http";

@Injectable({
  providedIn: 'root'
})
export class RepositoryService {
  private repositories: Repository[] = [];
  constructor(
    private readonly http: HttpClient,
  ) {
    let current_page = 0;
    let new_repos : Repository[] = [];
    do {
      new_repos = [];
      let http_params = new HttpParams().set('page', current_page).set('per_page', 100);
      this.http.get<Repository[]>(`https://api.${API_URL}/repositories/`, {
        withCredentials: true,
        params: http_params
      }).forEach((value) => {
        new_repos.concat(value);
      }).finally()
      this.repositories.concat(new_repos);
      console.log("received {} with page {}", new_repos.length, current_page);
      current_page += 1;
    } while (new_repos.length > 0);
  }

  public getRepositories() : Observable<Repository[]> {
    return of(this.repositories);
  }
}
