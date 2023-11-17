import { Injectable } from '@angular/core';
import {Observable, of} from "rxjs";
import {Project} from "./project.domain";

@Injectable({
  providedIn: 'root'
})
export class ProjectService {

  constructor() { }


  public getProjects() : Observable<Project[]> {
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
}
