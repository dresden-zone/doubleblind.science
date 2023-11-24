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
  private repositories: Repository[] = [];
  constructor(
    private readonly http: HttpClient,
  ) {
    this.getRepoRec(0).subscribe(value => {
      this.repositories = value;
    });
  }

  private getRepoRec(current_page: number): Observable<Repository[]> {
    let http_params = new HttpParams().set('page', current_page).set('per_page', 100);
    return this.http.get<Repository[]>(`https://api.${API_URL}/repositories/`, {
      withCredentials: true,
      params: http_params
    }).pipe(switchMap(value => {
      return this.getRepoRec(current_page + 1).pipe(map(value_2 => [...value,...value_2]));
    }))
  }

  public getRepositories() : Observable<Repository[]> {
    return of(this.repositories);
  }
}
