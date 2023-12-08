import { Injectable } from '@angular/core';
import {map, Observable, of, switchMap} from "rxjs";
import {Project} from "./project.domain";
import {Repository, SearchResult} from "./repository.domain";
import {API_URL} from "./api.domain";
import {HttpClient, HttpParams} from "@angular/common/http";

@Injectable({
  providedIn: 'root'
})
export class RepositoryService {
  constructor(
    private readonly http: HttpClient,
  ) {
  }

  public getRepositories(search_term: string) : Observable<Repository[]> {
    let http_params = new HttpParams().set('search', search_term);
    return this.http.get<SearchResult>(`https://api.${API_URL}/repositories/`, {
      withCredentials: true,
      params: http_params
    }).pipe(map(value => value.items))
  }
}
