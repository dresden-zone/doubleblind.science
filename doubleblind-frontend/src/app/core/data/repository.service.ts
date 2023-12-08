import { Injectable } from '@angular/core';
import {map, Observable, of, switchMap} from "rxjs";
import {Project} from "./project.domain";
import {Repository} from "./repository.domain";
import {API_URL} from "./api.domain";
import {HttpClient, HttpParams} from "@angular/common/http";

@Injectable({
  providedIn: 'root'
})
export class RepositoryService {
  private repositories  = this.getRepoRec(-1);
  constructor(
    private readonly http: HttpClient,
  ) {
  }

  private getRepoRec(current_page: number): Observable<Repository[]> {
    let http_params = new HttpParams().set('page', current_page).set('per_page', 100);
    return this.http.get<Repository[]>(`https://api.${API_URL}/repositories/`, {
      withCredentials: true,
      params: http_params
    }).pipe(switchMap(value => {
      if (value.length == 0) {
        return of([])
      } else {
        return this.getRepoRec(current_page + 1).pipe(map(value_2 => [...value,...value_2]));
      }
    }))
  }

  public getRepositories() : Observable<Repository[]> {
    return this.repositories;
  }
}
