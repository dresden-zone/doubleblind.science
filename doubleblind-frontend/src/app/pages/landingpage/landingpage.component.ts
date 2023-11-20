import { Component } from '@angular/core';
import { CommonModule } from '@angular/common';
import {ButtonComponent} from "@feel/form";
import {CardComponent} from "../../core/components/card/card.component";
import {IconGithubComponent} from "../../core/icons/icon-github/icon-github.component";
import {IconTudComponent} from "../../core/icons/icon-tud/icon-tud.component";

@Component({
  selector: 'app-landingpage',
  standalone: true,
  imports: [CommonModule, ButtonComponent, CardComponent, IconGithubComponent, IconTudComponent],
  templateUrl: './landingpage.component.html',
  styleUrl: './landingpage.component.scss'
})
export class LandingpageComponent {
  protected call() {
    location.href='https://api.science.tanneberger.me/auth/github/login';
  }
}
