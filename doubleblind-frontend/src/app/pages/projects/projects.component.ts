import { Component } from '@angular/core';
import { CommonModule } from '@angular/common';
import {ProjectService} from "../../core/data/project.service";
import {IconTrashComponent} from "../../core/icons/icon-trash/icon-trash.component";
import {ButtonComponent} from "@feel/form";
import {CardComponent} from "../../core/components/card/card.component";

@Component({
  selector: 'app-projects',
  standalone: true,
  imports: [CommonModule, IconTrashComponent, ButtonComponent, CardComponent],
  templateUrl: './projects.component.html',
  styleUrl: './projects.component.scss'
})
export class ProjectsComponent {
  protected readonly projects = this.projectService.getProjects();
  constructor(
    private readonly projectService:ProjectService
  ) {

  }
}
