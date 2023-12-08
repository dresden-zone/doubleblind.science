import { Injectable } from '@angular/core';
import {map, Observable, of} from "rxjs";
import {Project} from "./project.domain";
import {HttpClient} from "@angular/common/http";
import {API_URL} from "./api.domain";

@Injectable({
  providedIn: 'root'
})
export class ProjectService {

  constructor(
    private readonly http: HttpClient
  ) { }


  public getUserProjects() : Observable<Project[]> {
    return this.http.get<Project[]>(`https://api.${API_URL}/project/`,{
      withCredentials: true
    })
  }
  public getFeaturedProjects() : Observable<Project[]> {
    //return this.http.get<Project[]>(`https://api.${API_URL}/project`);
    return of([
      {
        id: "UUID-1",
        repo: "https://github.com/tanneberger/test-1",
        owner: "UID-1",
        name: "Troll1",
        last_update: "2023-08-24T20:21Z "
      },
      {
        id: "UUID-2",
        repo: "https://github.com/MarcelCoding/Foobaar",
        owner: "UID-1",
        name: "SeriousHihi",
        last_update: "2023-10-24T20:21Z "
      },
      {
        id: "UUID-3",
        repo: "https://github.com/robertro",
        owner: "UID-1",
        name: "FooBar",
        last_update: "2013-12-24T18:21Z "
      }
    ])
  }

  public create(name: string, github_name: string): Observable<void> {
    console.log("creating project with " + name + " and repo " + github_name);
    return this.http.post(`https://api.${API_URL}/project/`, {
      domain: name,
      github_name: github_name
    }, {
      withCredentials: true
    }).pipe(map(() => void 0));
  }
}
